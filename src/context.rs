use actix::Actor;
use actix_web_actors::ws::WebsocketContext;
use serde::{Deserialize, Serialize};

impl<A, T> Msg<T> for WebsocketContext<A>
where
    A: Actor<Context = Self>,
    T: Serialize,
{
    fn full(&mut self, code: Code, msg: T) {
        let data = Data(code.code(), msg);
        self.text(serde_json::to_string(&data).unwrap());
    }

    fn sys(&mut self, msg: T) {
        let data = Data(Code::Sys.code(), msg);
        self.text(serde_json::to_string(&data).unwrap());
    }

    fn progress(&mut self, msg: T) {
        let data = Data(Code::Progress.code(), msg);
        self.text(serde_json::to_string(&data).unwrap());
    }

    fn msg(&mut self, user: String, msg: String) {
        let data = Data(Code::Msg.code(), MsgData(user, msg));
        self.text(serde_json::to_string(&data).unwrap());
    }
}

pub trait Msg<T>
where
    T: Serialize,
{
    fn full(&mut self, code: Code, msg: T);
    fn sys(&mut self, msg: T);
    fn progress(&mut self, msg: T);
    fn msg(&mut self, user: String, msg: String);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data<T>(pub i32, pub T)
where
    T: Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgData(pub String, pub String);

impl<T> Data<T>
where
    T: Serialize,
{
    pub fn full(code: Code, msg: T) -> String {
        let data = Data(code.code(), msg);
        serde_json::to_string(&data).unwrap()
    }

    pub fn sys(msg: T) -> String {
        let data = Data(Code::Sys.code(), msg);
        serde_json::to_string(&data).unwrap()
    }

    pub fn progress(msg: T) -> String {
        let data = Data(Code::Progress.code(), msg);
        serde_json::to_string(&data).unwrap()
    }
}
impl Data<MsgData> {
    pub fn msg(user: String, msg: String) -> String {
        let data = Data(Code::Msg.code(), MsgData(user, msg));
        serde_json::to_string(&data).unwrap()
    }
}

pub enum Code {
    Msg,
    Sys,
    Progress,
    Roomer,
    Share
}
impl Code {
    pub fn code(self) -> i32{
        match self {
            Code::Msg => 0,
            Code::Sys => 1,
            Code::Progress => 2,
            Code::Roomer => 3,
            Code::Share => 4,
        }
    }
}