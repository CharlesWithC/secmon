use serde::Serialize;

#[derive(Serialize)]
pub struct Embed {
    pub title: String,
    pub description: String,
    pub fields: Vec<EmbedField>,
}

impl Embed {
    pub fn new() -> Embed {
        Embed {
            title: String::new(),
            description: String::new(),
            fields: Vec::<EmbedField>::new(),
        }
    }
}

#[derive(Serialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

#[derive(Serialize)]
pub struct Webhook {
    pub content: String,
    pub embeds: Vec<Embed>,
}
