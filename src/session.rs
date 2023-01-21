use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web_actors::ws;

use crate::{
    context::{Code, Msg},
    server::{self, Login},
};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct WsChatSession {
    /// unique session id
    pub id: usize,

    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    pub hb: Instant,

    /// joined room
    pub room: String,

    /// Chat server
    pub addr: Addr<server::ChatServer>,
}

impl WsChatSession {
    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send(server::Disconnect { id: act.id });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with ChatServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.addr
            .send(server::Connect {
                addr: addr.recipient(),
                user: (None, None),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.addr.do_send(server::Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<server::Message> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        log::debug!("WEBSOCKET MESSAGE: {msg:?}");
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();
                // we check for /sss type of messages
                println!("{}", m);
                if m.starts_with('/') {
                    let v: Vec<&str> = m.splitn(2, ' ').collect();
                    match v[0] {
                        "/list" => {
                            // Send ListRooms message to chat server and wait for
                            // response
                            println!("List rooms");
                            self.addr
                                .send(server::ListRooms)
                                .into_actor(self)
                                .then(|res, _, ctx| {
                                    match res {
                                        Ok(rooms) => {
                                            for room in rooms {
                                                ctx.sys(room);
                                            }
                                        }
                                        _ => println!("Something is wrong"),
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx)
                        }
                        "/join" => {
                            if v.len() == 2 {
                                self.room = v[1].to_owned();
                                self.addr
                                    .send(server::Join {
                                        id: self.id,
                                        name: self.room.clone(),
                                    })
                                    .into_actor(self)
                                    .then(|res, _, ctx| {
                                        if let Ok(v) = res {
                                            ctx.full(Code::Roomer, v);
                                        }
                                        fut::ready(())
                                    })
                                    .wait(ctx);
                                ctx.sys("已加入房间".to_owned());
                            } else {
                                ctx.sys("!!! room name is required".to_owned());
                            }
                        }
                        "/count" => {
                            self.addr
                                .send(server::Count)
                                .into_actor(self)
                                .then(|res, _, ctx| {
                                    if let Ok(v) = res {
                                        ctx.sys(format!("[Online] {}", v));
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx);
                        }
                        "/members" => {
                            let data = server::ListMembers {
                                room_id: self.room.clone(),
                            };
                            self.addr
                                .send(data)
                                .into_actor(self)
                                .then(|res, _, ctx| {
                                    if let Ok(Some(v)) = res {
                                        ctx.full(Code::Members, v);
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx);
                        }
                        "/progress" => {
                            if v.len() == 2 {
                                let value: Vec<&str> = v[1].splitn(2, "\n").collect();
                                if value.len() == 2 {
                                    self.addr
                                        .send(server::Progress {
                                            id: self.id,
                                            progress: value[0].to_owned(),
                                            speed: value[1].to_owned(),
                                            room: self.room.clone(),
                                        })
                                        .into_actor(self)
                                        .then(|res, _, ctx| {
                                            if let Ok(Some(v)) = res {
                                                ctx.full(v.0, v.1);
                                            }
                                            fut::ready(())
                                        })
                                        .wait(ctx);
                                } else {
                                    ctx.sys("!!! speed is required".to_owned());
                                }
                            } else {
                                ctx.sys("!!! Progress is required".to_owned());
                            }
                        }
                        "/login" => {
                            if v.len() == 2 {
                                let value: Vec<&str> = v[1].splitn(2, "\n").collect();
                                if value.len() == 2 {
                                    let name = Some(value[0].to_owned());
                                    let avatar = Some(value[1].to_owned());
                                    self.addr.do_send(Login(self.id, name, avatar));
                                } else {
                                    ctx.sys("!!! avatar is required".to_owned());
                                }
                            } else {
                                ctx.sys("!!! name is required".to_owned());
                            }
                        }
                        "/share" => {
                            if v.len() == 2 {
                                let msg = Some(v[1].to_owned()).unwrap();
                                // send message to chat server
                                self.addr.do_send(server::FullMessage {
                                    id: self.id,
                                    code: Code::Share,
                                    msg,
                                    room: self.room.clone(),
                                })
                            } else {
                                ctx.sys("!!! share value is required".to_owned());
                            }
                        }
                        "/speed" => {
                            if v.len() == 2 {
                                let msg = Some(v[1].to_owned()).unwrap();
                                // send message to chat server
                                self.addr.do_send(server::FullMessage {
                                    id: self.id,
                                    code: Code::Speed,
                                    msg,
                                    room: self.room.clone(),
                                })
                            } else {
                                ctx.sys("!!! share value is required".to_owned());
                            }
                        }
                        "/msg" => {
                            if v.len() == 2 {
                                let msg = Some(v[1].to_owned()).unwrap();
                                // send message to chat server
                                self.addr.do_send(server::ClientMessage {
                                    id: self.id,
                                    msg,
                                    room: self.room.clone(),
                                })
                            } else {
                                ctx.sys("!!! msg is required".to_owned());
                            }
                        }
                        _ => ctx.sys(format!("!!! unknown command: {m:?}")),
                    }
                }
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
