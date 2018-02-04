use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub name: String,
    pub id_board: String,
    pub id_list: String,
    pub id_members: Vec<String>
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub avatar_hash: String,
    pub full_name: String,
    pub initials: String,
    pub username: String
}

#[derive(Deserialize)]
pub struct Action {
    pub id: String,
    pub data: Value,
    pub date: String, // TODO: Switch to using DateTime<Utc> later
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(rename = "idMemberCreator")]
    pub creator_id: String,
    #[serde(rename = "memberCreator")]
    pub creator: Member
}