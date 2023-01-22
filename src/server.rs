//! `ChatServer` is an actor. It maintains list of connection client session.
//! And manages available rooms. Peers send messages to other peers in same
//! room through `ChatServer`.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use crate::context::{Code, Data};

/// Chat server sends this messages to session
#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

/// Message for chat server communications

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub user: (Option<String>, Option<String>),
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// Send message to specific room
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
    /// Room name
    pub room: String,
}

/// List of available rooms
pub struct ListRooms;
pub struct Count;

impl actix::Message for ListRooms {
    type Result = Vec<String>;
}

impl actix::Message for Count {
    type Result = String;
}

/// `ChatServer` manages chat rooms and responsible for coordinating chat session.
///
/// Implementation is very naïve.
#[derive(Debug)]
pub struct ChatServer {
    sessions: HashMap<usize, (Recipient<Message>, (Option<String>, Option<String>))>,
    rooms: HashMap<String, Room>,
    rng: ThreadRng,
    visitor_count: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Room {
    pub roomer: usize,
    pub members: HashSet<usize>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoomInfo {
    pub roomer: User,
    pub members: Vec<User>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: usize,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

impl Room {
    pub fn new(roomer: usize) -> Room {
        let mut set = HashSet::new();
        set.insert(roomer);
        Room {
            roomer,
            members: set,
        }
    }
}

impl ChatServer {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> ChatServer {
        // default room
        let rooms = HashMap::new();
        // rooms.insert("main".to_owned(), HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
            visitor_count,
        }
    }
}

impl ChatServer {
    /// Send message to all users in the room
    fn send_message(&self, room: &str, message: &str, skip_id: usize) {
        if let Some(Room { roomer: _, members }) = self.rooms.get(room) {
            for id in members {
                if *id != skip_id {
                    if let Some((addr, _)) = self.sessions.get(id) {
                        addr.do_send(Message(message.to_owned()));
                    }
                }
            }
        }
    }
    /// Send message to specific user
    fn send(&self, message: &str, uid: usize) {
        if let Some((addr, _)) = self.sessions.get(&uid) {
            addr.do_send(Message(message.to_owned()));
        }
    }
}

/// Make actor from `ChatServer`
impl Actor for ChatServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("{:?} joined", msg.addr);

        // notify all users in same room
        // self.send_message("main", format!("{:?} joined",msg.addr).as_str(), 0);

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, (msg.addr, msg.user));

        // auto join session to main room
        // self.rooms
        //     .entry("main".to_owned())
        //     .or_insert_with(HashSet::new)
        //     .insert(id);

        self.visitor_count.fetch_add(1, Ordering::SeqCst);
        // self.send_message("main", &format!("Total visitors {count}"), 0);

        // send id back
        id
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Login(pub usize, pub Option<String>, pub Option<String>);
impl Handler<Login> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Login, _: &mut Context<Self>) -> Self::Result {
        self.sessions
            .entry(msg.0)
            .and_modify(|(_, (name, avatar))| {
                *name = msg.1;
                *avatar = msg.2;
            });
        println!("User Login:{:?}", self.get_user(msg.0))
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let user = self.get_user(msg.id);
        println!("{:?} disconnected", user.name.clone());

        let mut rooms: Vec<String> = Vec::new();
        let mut empty_rooms: Vec<String> = Vec::new();
        let mut new_roomer: Option<_> = None;
        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all rooms
            for (name, Room { roomer, members }) in &mut self.rooms {
                if members.remove(&msg.id) {
                    rooms.push(name.to_owned());
                }
                if roomer == &msg.id {
                    // 转让房主
                    // 后期根据房间设置确认是否转让
                    if let Some(u) = members.clone().into_iter().next() {
                        *roomer = u;
                        new_roomer = Some(u);
                    } else {
                        // 空房间，删除
                        empty_rooms.push(name.to_owned());
                    }
                }
            }
        }
        // 给新房主发消息
        if let Some(roomer) = new_roomer {
            self.send(Data::full(Code::Roomer, true).as_str(), roomer);
        }
        self.visitor_count.fetch_min(1, Ordering::SeqCst);
        // send message to other users
        for room in rooms {
            self.send_message(
                &room,
                &Data::sys(format!(
                    "{} 退出房间",
                    user.name.clone().unwrap_or(msg.id.to_string())
                )),
                0,
            );
        }
        for room in empty_rooms {
            self.rooms.remove(room.as_str());
        }
    }
}

/// Handler for Message message.
impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        let user = self.get_user(msg.id);
        self.send_message(
            &msg.room,
            &Data::msg(
                user.id.to_string(),
                msg.msg,
            ),
            msg.id,
        );
    }
}

/// Send message to specific room
#[derive(Message)]
#[rtype(result = "()")]
pub struct FullMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub code: Code,
    pub msg: String,
    /// Room name
    pub room: String,
}
/// Handler for Message message.
impl Handler<FullMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: FullMessage, _: &mut Context<Self>) {
        self.send_message(&msg.room, &Data::full(msg.code, msg.msg), msg.id);
    }
}

#[derive(Message)]
#[rtype(result = "Option<(Code,String)>")]
pub struct Progress {
    /// Client ID
    pub id: usize,
    /// Room name
    pub room: String,
    /// Progress
    pub progress: String,
    pub speed: String,
}
/// Handler for Message message.
impl Handler<Progress> for ChatServer {
    type Result = Option<(Code, String)>;

    fn handle(&mut self, msg: Progress, _: &mut Context<Self>) -> Self::Result {
        match self.rooms.get(msg.room.as_str()) {
            Some(v) => {
                if v.members.len() > 0 {
                    if v.roomer == msg.id {
                        // 房主,允许广播进度
                        self.send_message(
                            &msg.room,
                            &Data::progress((msg.progress, msg.speed)),
                            msg.id,
                        );
                        None
                    } else {
                        Some((Code::Sys, "NOT_ROOMER".to_string()))
                    }
                } else {
                    Some((Code::Sys, "DEAD_ROOM".to_string()))
                }
            }
            None => Some((Code::Sys, "ROOM_NOT_EXIST".to_string())),
        }
    }
}

/// Handler for `ListRooms` message.
impl Handler<ListRooms> for ChatServer {
    type Result = MessageResult<ListRooms>;

    fn handle(&mut self, _: ListRooms, _: &mut Context<Self>) -> Self::Result {
        let mut rooms = Vec::new();

        for key in self.rooms.keys() {
            rooms.push(key.to_owned())
        }

        MessageResult(rooms)
    }
}

#[derive(Message)]
#[rtype(result = "Option<RoomInfo>")]
pub struct ListMembers {
    pub room_id: String,
}

/// Handler for `ListRooms` message.
impl Handler<ListMembers> for ChatServer {
    type Result = Option<RoomInfo>;

    fn handle(
        &mut self,
        ListMembers { room_id }: ListMembers,
        _: &mut Context<Self>,
    ) -> Self::Result {
        if let Some(room) = self.rooms.get(&room_id) {
            Some(RoomInfo {
                roomer: self.get_user(room.roomer),
                members: room.members.iter().map(|id| self.get_user(*id)).collect(),
            })
        } else {
            None
        }
    }
}

impl ChatServer {
    fn get_user(&self, id: usize) -> User {
        if let Some((_, (name, avatar))) = self.sessions.get(&id) {
            User {
                id,
                name: name.to_owned(),
                avatar: avatar.to_owned(),
            }
        } else {
            User {
                id,
                name: None,
                avatar: None,
            }
        }
    }
}
/// Handler for `ListRooms` message.
impl Handler<Count> for ChatServer {
    type Result = MessageResult<Count>;

    fn handle(&mut self, _: Count, _: &mut Context<Self>) -> Self::Result {
        MessageResult(format!("{:?}", self.visitor_count))
    }
}

/// Join room, if room does not exists create new one.
#[derive(Message)]
#[rtype(result = "bool")]
pub struct Join {
    /// Client ID
    pub id: usize,

    /// Room name
    pub name: String,
}
/// Join room, send disconnect message to old room
/// send join message to new room
impl Handler<Join> for ChatServer {
    type Result = bool;

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) -> Self::Result {
        let Join { id, name } = msg;
        let user = self.get_user(id);
        let mut rooms = Vec::new();
        // remove session from all rooms
        for (n, Room { roomer: _, members }) in &mut self.rooms {
            if members.remove(&id) {
                rooms.push(n.to_owned());
            }
        }
        // send message to other users
        for room in rooms {
            self.send_message(
                &room,
                &Data::sys(format!(
                    "{} 退出房间",
                    user.name.clone().unwrap_or(id.to_string())
                )),
                0,
            );
        }
        let mut roomer = false;
        self.rooms
            .entry(name.clone())
            .and_modify(|Room { roomer: _, members }| {
                members.insert(id);
            })
            .or_insert_with(|| {
                roomer = true;
                Room::new(id)
            });

        self.send_message(
            &name,
            &Data::sys(format!(
                "{} 进入房间",
                user.name.clone().unwrap_or(id.to_string())
            )),
            id,
        );
        roomer
    }
}
