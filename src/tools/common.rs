pub type Res = (String, Option<String>);

pub fn ok() -> Res { ("ok".to_string(), None) }
pub fn e400(msg: impl ToString) -> Res { ("E400".to_string(), Some(msg.to_string())) }
pub fn e404(msg: impl ToString) -> Res { ("E404".to_string(), Some(msg.to_string())) }
pub fn e408(msg: impl ToString) -> Res { ("E408".to_string(), Some(msg.to_string())) }
pub fn e409(msg: impl ToString) -> Res { ("E409".to_string(), Some(msg.to_string())) }
pub fn e500(msg: impl ToString) -> Res { ("E500".to_string(), Some(msg.to_string())) }

pub fn is_error(s: &str) -> bool {
    matches!(s, "E400" | "E404" | "E408" | "E409" | "E500")
}
