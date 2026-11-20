//! Builds the weekly-plan prompt and parses the ideas the AI sends back.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub channel_id: String,
    pub channel_name: String,
    pub platform: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest {
    pub provider: String,
    pub offer_id: i64,
    pub slots: Vec<Slot>,
    pub with_images: bool,
    /// The topic every idea should be about. Blank lets the agent pick
    /// angles from the offer alone.
    #[serde(default)]
    pub topic_title: Option<String>,
    #[serde(default)]
    pub topic_description: Option<String>,
    /// The idea skill picked for each planner channel id. Channels left
    /// out use the default skill.
    #[serde(default)]
    pub channel_skills: HashMap<String, String>,
    /// Draw each idea's cover image with the image API after planning,
    /// instead of only writing the image brief.
    #[serde(default)]
    pub generate_images: bool,
}

/// A skill as it goes into the prompt.
pub struct PlanSkill {
    pub name: String,
    pub content: String,
}

/// The plan's topic, if the user gave one.
pub struct Topic<'a> {
    pub title: &'a str,
    pub description: &'a str,
}

impl PlanRequest {
    pub fn topic(&self) -> Option<Topic<'_>> {
        let title = self.topic_title.as_deref().map(str::trim).unwrap_or_default();
        let description = self.topic_description.as_deref().map(str::trim).unwrap_or_default();
        (!title.is_empty() || !description.is_empty()).then_some(Topic { title, description })
    }
}

/// One idea as the AI returns it.
#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct Drafted {
    pub title: String,
    pub short_notes: String,
    pub pain: String,
    pub pattern: String,
    pub keyword: String,
    pub tag: String,
    pub draft: String,
    pub image_prompt: Option<String>,
}

/// `slot_skills[i]` is the index in `skills` of the skill slot `i` follows.
#[allow(clippy::too_many_arguments)]
pub fn build_prompt(
    skills: &[PlanSkill],
    slot_skills: &[usize],
    company: &Value,
    offer: &Value,
    slots: &[Slot],
    topic: Option<&Topic>,
    with_images: bool,
    existing_titles: &[String],
) -> String {
    let mut prompt = String::from(
        "You are the editorial agent for a B2B studio. Plan next week's social content ideas \
         for the offer below, one per post slot. Work only from the material in this message: do not use tools, \
         read files, or ask questions.\n\n",
    );

    let several_skills = skills.len() > 1;
    if several_skills {
        prompt.push_str(
            "# Writing skills (follow strictly)\n\nEach slot below names the skill its idea must follow. \
             Apply only that skill's rules to that idea.",
        );
        for skill in skills {
            prompt.push_str(&format!("\n\n## Skill: {}\n\n{}", skill.name.trim(), skill.content.trim()));
        }
    } else if let Some(skill) = skills.first() {
        prompt.push_str("# Writing skill (follow strictly)\n\n");
        prompt.push_str(skill.content.trim());
    }

    prompt.push_str("\n\n# Company\n\n");
    prompt.push_str(&serde_json::to_string_pretty(company).unwrap_or_default());

    prompt.push_str("\n\n# Focus offer (primary CTA and case angle)\n\n");
    prompt.push_str(&serde_json::to_string_pretty(offer).unwrap_or_default());

    if let Some(topic) = topic {
        prompt.push_str("\n\n# Topic for this week (every idea must be about it)\n\n");
        if !topic.title.is_empty() {
            prompt.push_str(&format!("Title: {}\n", topic.title));
        }
        if !topic.description.is_empty() {
            prompt.push_str(&format!("Description: {}\n", topic.description));
        }
        prompt.push_str("Tie each idea to this topic while still selling the focus offer.");
    }

    prompt.push_str("\n\n# Slots to fill\n\nWrite exactly one idea per slot, in this order:\n");
    for (index, slot) in slots.iter().enumerate() {
        prompt.push_str(&format!("{}. {} ({})", index + 1, slot.channel_name, slot.platform));
        if several_skills && let Some(skill) = slot_skills.get(index).and_then(|&i| skills.get(i)) {
            prompt.push_str(&format!(" - skill: {}", skill.name.trim()));
        }
        prompt.push('\n');
    }
    prompt.push_str(
        "\nFit each idea to its platform: LinkedIn is long-form and opinionated, X is one sharp \
         point or a thread opener, Facebook is short and conversational, Website is an SEO \
         article angle. Every idea must take a different angle on the offer.\n",
    );

    if !existing_titles.is_empty() {
        prompt.push_str("\n# Already planned (do not repeat these angles)\n\n");
        for title in existing_titles {
            prompt.push_str(&format!("- {title}\n"));
        }
    }

    prompt.push_str(&format!(
        "\n# Output\n\nReply with ONLY a JSON array of exactly {} objects, one per slot in slot \
         order. No markdown fences, no commentary. Each object has these string fields:\n\
         - \"title\": the hook / headline\n\
         - \"short_notes\": one sentence on the angle and who it targets\n\
         - \"pain\": the audience pain it addresses\n\
         - \"pattern\": the post pattern (e.g. Contrarian breakdown, Cost dissection, Before/After benchmark, Step-by-step framework, Founder lesson)\n\
         - \"keyword\": the search-intent keyword to target\n\
         - \"tag\": a 1-2 word topic tag\n\
         - \"draft\": a ready-to-edit draft of the post for that platform\n",
        slots.len()
    ));
    if with_images {
        prompt.push_str(
            "- \"image_prompt\": a brief for an on-brand cover image for this post (subject, composition, style)\n",
        );
    }

    prompt
}

/// Pulls the JSON array of ideas out of the CLI output, tolerating code
/// fences or chatter around it.
pub fn parse_ideas(output: &str) -> Result<Vec<Drafted>, String> {
    for (index, _) in output.match_indices('[') {
        let mut stream = serde_json::Deserializer::from_str(&output[index..]).into_iter::<Vec<Drafted>>();
        if let Some(Ok(ideas)) = stream.next() {
            let ideas: Vec<Drafted> = ideas.into_iter().filter(|idea| !idea.title.trim().is_empty()).collect();
            if !ideas.is_empty() {
                return Ok(ideas);
            }
        }
    }

    let preview: String = output.trim().chars().take(400).collect();
    Err(format!("The AI didn't return ideas in the expected format. It said:\n{preview}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bare_json_array() {
        let ideas = parse_ideas(r#"[{"title":"A","draft":"Body [1]","keyword":"k"}]"#).unwrap();
        assert_eq!(ideas.len(), 1);
        assert_eq!(ideas[0].title, "A");
        assert_eq!(ideas[0].draft, "Body [1]");
        assert_eq!(ideas[0].image_prompt, None);
    }

    #[test]
    fn parses_an_array_wrapped_in_fences_and_chatter() {
        let output = "Here you go [as asked]:\n```json\n[{\"title\":\"A\"},{\"title\":\"B\",\"image_prompt\":\"desk\"}]\n```\nDone.";
        let ideas = parse_ideas(output).unwrap();
        assert_eq!(ideas.len(), 2);
        assert_eq!(ideas[1].image_prompt.as_deref(), Some("desk"));
    }

    #[test]
    fn rejects_output_without_ideas() {
        let error = parse_ideas("Sorry, I need more details. []").unwrap_err();
        assert!(error.contains("Sorry, I need more details."));
    }

    #[test]
    fn prompt_lists_every_slot_and_only_asks_for_images_when_wanted() {
        let slots = vec![Slot {
            channel_id: "x_profile".into(),
            channel_name: "X · account".into(),
            platform: "X".into(),
        }];
        let offer = serde_json::json!({ "name": "AI Automation Sprint" });

        let skills = one_skill();
        let text_only = build_prompt(&skills, &[0], &Value::Null, &offer, &slots, None, false, &[]);
        let with_images = build_prompt(&skills, &[0], &Value::Null, &offer, &slots, None, true, &["Old idea".into()]);

        assert!(text_only.contains("# Writing skill (follow strictly)\n\nSKILL"));
        assert!(text_only.contains("1. X · account (X)\n"));
        assert!(text_only.contains("AI Automation Sprint"));
        assert!(!text_only.contains("image_prompt"));
        assert!(with_images.contains("image_prompt"));
        assert!(with_images.contains("- Old idea"));
        assert!(!text_only.contains("# Topic"));
    }

    #[test]
    fn prompt_includes_the_topic_when_given_and_blank_topics_are_ignored() {
        let request = |title: &str, description: &str| PlanRequest {
            provider: "claude".into(),
            offer_id: 1,
            slots: Vec::new(),
            with_images: false,
            topic_title: Some(title.into()),
            topic_description: Some(description.into()),
            channel_skills: HashMap::new(),
            generate_images: false,
        };
        assert!(request("  ", "").topic().is_none());

        let focused = request(" Black Friday prep ", "Scaling stores for traffic spikes");
        let prompt =
            build_prompt(&one_skill(), &[], &Value::Null, &Value::Null, &[], focused.topic().as_ref(), false, &[]);

        assert!(prompt.contains("Title: Black Friday prep\n"));
        assert!(prompt.contains("Description: Scaling stores for traffic spikes\n"));
    }

    #[test]
    fn several_skills_are_each_included_and_named_on_their_slots() {
        let slot = |id: &str, name: &str, platform: &str| Slot {
            channel_id: id.into(),
            channel_name: name.into(),
            platform: platform.into(),
        };
        let slots = vec![slot("linkedin_personal", "LinkedIn · Personal profile", "LinkedIn"), slot("x_profile", "X · account", "X")];
        let skills = vec![
            PlanSkill { name: "Long form".into(), content: "LONG RULES".into() },
            PlanSkill { name: "Punchy".into(), content: "SHORT RULES".into() },
        ];

        let prompt = build_prompt(&skills, &[0, 1], &Value::Null, &Value::Null, &slots, None, false, &[]);

        assert!(prompt.contains("## Skill: Long form\n\nLONG RULES"));
        assert!(prompt.contains("## Skill: Punchy\n\nSHORT RULES"));
        assert!(prompt.contains("1. LinkedIn · Personal profile (LinkedIn) - skill: Long form\n"));
        assert!(prompt.contains("2. X · account (X) - skill: Punchy\n"));
    }

    fn one_skill() -> Vec<PlanSkill> {
        vec![PlanSkill { name: "Default".into(), content: "SKILL".into() }]
    }
}
