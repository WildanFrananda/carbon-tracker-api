use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;

#[derive(Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

pub async fn get_ai_recommendation(user_data: String) -> Result<String, Box<dyn Error>> {
    let api_key = env::var("GROQ_API_KEY")?;
    let api_url = env::var("GROQ_API_URL")?;

    let client = reqwest::Client::new();

    let prompt = format!(
        "You're a savage Gen Z eco-expert. User data: {}. \
        Give 2 ultra-short, practical tips to slash their carbon footprint. \
        Use Gen Z slang (fr, no cap, bestie, etc.), keep it concise, and strictly in English. \
        No yapping, just vibes and facts.",
        user_data
    );

    let request = GroqRequest {
        model: "llama-3.1-8b-instant".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "You are a savage Gen Z environmental expert who speaks in short, trendy English slang. No long sentences.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    };

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await?;
        return Err(format!("API Error ({}): {}", status, error_text).into());
    }
    let groq_res = response.json::<GroqResponse>().await?;

    return Ok(groq_res.choices[0].message.content.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[tokio::test]
    async fn test_groq_integration() {
        dotenv().ok();
        let test_data = "Transport: 25.5kg CO2, Food: 5.0kg CO2, Energy: 10.2kg CO2".to_string();

        println!("\n--- 🔍 DEBUG AI SERVICE ---");
        match get_ai_recommendation(test_data).await {
            Ok(response) => {
                println!("✅ RESPONS AI DITERIMA:\n{}", response);
            }
            Err(e) => {
                println!("❌ ERROR DETECTED: {:?}", e);
            }
        }
    }
}
