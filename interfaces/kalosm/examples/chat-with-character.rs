use kalosm::language::*;

#[tokio::main]
async fn main() {
    let description = prompt_input("What is your character like? ").unwrap();
    let character_name = prompt_input("What is your character's name? ").unwrap();
    let character_description =
        format!("{description} The assistant will never reveal that it is an AI or assistant.");

    let model = Llama::new_chat().await.unwrap();
    // Create constraints that parses (Responding as <character_name>) and then stops on the end of the assistant's response
    let constraints = LiteralParser::new(format!("(Responding as {}) ", character_name))
        .then(model.default_assistant_constraints().unwrap());
    // Create a chat session with the model and the constraints
    let mut chat = Chat::builder(model)
        .with_system_prompt(character_description)
        .constrain_response(move |_history| constraints.clone())
        .build();

    // Chat with the user
    loop {
        let mut output_stream = chat.add_message(prompt_input("\n> ").unwrap());
        print!("Bot: ");
        output_stream.to_std_out().await.unwrap();
    }
}
