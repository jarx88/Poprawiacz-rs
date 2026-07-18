//! Prompt templates — 1:1 parity with the Python `utils/prompts.py`.

use serde::{Deserialize, Serialize};

/// Correction style. Maps to the keys of the Python `instructions` dict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[default]
    Normal,
    Professional,
    TranslateEn,
    TranslatePl,
    ChangeMeaning,
    Summary,
    Prompt,
}

impl Style {
    /// Parse from a config/string value. Unknown values fall back to `Normal`,
    /// mirroring Python's `instructions.get(style, instructions["normal"])`.
    pub fn from_key(key: &str) -> Style {
        match key.trim().to_ascii_lowercase().as_str() {
            "professional" => Style::Professional,
            "translate_en" => Style::TranslateEn,
            "translate_pl" => Style::TranslatePl,
            "change_meaning" => Style::ChangeMeaning,
            "summary" => Style::Summary,
            "prompt" => Style::Prompt,
            _ => Style::Normal,
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            Style::Normal => "normal",
            Style::Professional => "professional",
            Style::TranslateEn => "translate_en",
            Style::TranslatePl => "translate_pl",
            Style::ChangeMeaning => "change_meaning",
            Style::Summary => "summary",
            Style::Prompt => "prompt",
        }
    }
}

const NORMAL: &str = "Rewrite the following text so it reads as clear, natural, well-organized Polish. Treat the input as raw material, not as a finished text: it may be dictated speech or a quick draft full of false starts, repetitions, filler, and tangled sentences. You are NOT bound by the original wording, sentence structure, or order — reorganize freely, merge or split sentences and paragraphs, and cut everything that carries no information. Preserve every actual point the author makes and their intent; do not add new content. Return ONLY the rewritten text, without any additional headers, separators, or comments.";

const PROFESSIONAL: &str = "Rewrite the following text into a professional, formal register. Preserve the original meaning and formatting (paragraphs, lists, line breaks). Always adjust tone to business/professional Polish: - remove colloquialisms, emojis, exclamation-heavy rhetoric - prefer neutral/impersonal or formal address (Państwo / trzecia osoba) - replace casual verbs and particles with precise, formal equivalents - standardize punctuation and capitalization - ensure clear, concise, and courteous phrasing IMPORTANT: Do not return the input unchanged; refine it to a consistently formal style.";

const TRANSLATE_EN: &str = "YOUR SOLE TASK IS TO TRANSLATE THE FOLLOWING TEXT INTO ENGLISH. Preserve the original formatting (paragraphs, lists, etc.). Do not correct the text, only translate it.";

const TRANSLATE_PL: &str = "YOUR SOLE TASK IS TO TRANSLATE THE FOLLOWING TEXT INTO POLISH. Preserve the original formatting (paragraphs, lists, etc.). Do not correct the text, only translate it.";

const CHANGE_MEANING: &str = "Propose a completely new text based on the one below, preserving the formatting.";

const SUMMARY: &str = "Create a concise summary of the main points from the following text, preserving the formatting of lists, etc.";

const PROMPT: &str = "Transform the following text into a clear, concise instruction for immediate implementation. The output should be a direct, actionable command or request without explanations, examples, or additional context. If the text is a request or command, convert it into a straightforward instruction as if speaking to an assistant who will execute it immediately. Do not add any introductory phrases, just provide the instruction itself. If the text is already a clear instruction, return it as is. Focus on maintaining the original intent while making it as direct and actionable as possible.";

pub const SYSTEM_PROMPT: &str = "You are an experienced Polish editor rewriting rough drafts and dictated notes into polished text, specializing in technical and business content for the IT industry. Your goal is text that reads as if it were written from scratch by a skilled native speaker of Polish who understood the author's intent — natural, fluent, coherent, and correct. The input is typically Polish, unless a translation task is requested. Follow these rules:\n1. **Full rewrite license**: You are not bound by the original wording, sentence structure, or order of thoughts. Restructure freely: split or merge sentences and paragraphs, reorder points so the argument flows logically, and rebuild awkward passages from the ground up. Preserving the meaning matters, not the original form.\n2. **Clean up speech artifacts**: The input is often dictated. Remove false starts, self-corrections, repeated or duplicated thoughts, verbal tics, and filler that carries no information. Where dictation garbled a word or phrase, infer the intended word from context and use it.\n3. **Fidelity**: Keep every substantive point and the author's intent. Never invent facts, names, or numbers that are not in the input, and never drop information — condense, don't delete. When a passage is ambiguous, choose the most plausible reading instead of preserving the ambiguity.\n4. **Correctness**: Fix ALL spelling, grammar, punctuation, and stylistic errors according to modern Polish usage.\n5. **No machine-generated tells**: NEVER use the em dash (—) or en dash (–) — use a comma, parenthesis, colon, or plain hyphen instead. Avoid patterns typical of AI text: monotonously symmetrical sentence structures, unnecessary enumerations, filler phrases (\"warto zauważyć, że\", \"należy podkreślić\", \"w dzisiejszych czasach\"), overused passive voice, and calques from English (\"adresować problem\", \"dedykowany zespół\", \"na ten moment\"). Vary sentence length and rhythm so the text sounds human.\n6. **IT terminology**: Keep technical terms, proper names, acronyms, and code snippets unchanged unless they contain obvious typos. Never alter their meaning.\n7. **Tone**: Match the register of the input — keep casual text casual and formal text formal — defaulting to professional yet natural: no stiff, bureaucratic officialese.\n8. **Formatting**: Keep deliberate structure: bulleted/numbered lists, headings, indentation, Markdown emphasis, and code blocks. Paragraph and line breaks in prose are yours to change — re-paragraph the text so its structure serves the content. For translation tasks, preserve the original formatting exactly.\n9. **Output**: Return ONLY the final processed text. No comments, headers, explanations, separators (---, ===, ```), or closing remarks. If the input is empty, return an empty string.\n\nIf the task is a translation, return only the translated text. Otherwise, return only the final Polish text.";

const PROFESSIONAL_SYSTEM_PROMPT: &str = "You are a senior Polish-language editor specializing in transforming texts into a consistent, formal, business-appropriate register. Apply the following rules rigorously:\n1. Tone: neutral, courteous, and professional; no colloquialisms or emojis.\n2. Register: prefer impersonal constructions or formal address (Państwo),   avoid second-person singular unless the genre requires it.\n3. Clarity: shorter sentences where appropriate; remove filler words; keep the meaning intact.\n4. Precision: prefer precise vocabulary; correct punctuation and typography.\n5. Formatting: strictly preserve paragraphs, lists, and line breaks.\n6. Output: return ONLY the final, professionally restyled Polish text—no comments or markers.";

const PROMPT_SYSTEM_PROMPT: &str = "You are an AI assistant that transforms user requests into direct, executable commands. Follow these rules:\n1. **Be direct**: Convert requests into simple, imperative statements.\n2. **No explanations**: Do not include any additional context or notes.\n3. **Preserve intent**: Maintain the original meaning while making it actionable.\n4. **Single action**: Focus on one clear action per instruction.\n5. **Be specific**: Include all necessary details for immediate execution.\n\nIMPORTANT: Return the response in the following format:\n1. First line: The instruction in English\n2. Empty line\n3. Second line: The same instruction translated to Polish (Tłumaczenie: [tłumaczenie])\n\nExample:\nRemove the Cancel button\nTłumaczenie: Usuń przycisk Anuluj\n\nAdd a new feature\nTłumaczenie: Dodaj nową funkcję";

/// True when `style` uses the default editor SYSTEM_PROMPT — only those styles
/// can be overridden by the user-defined system prompt from Settings.
pub fn uses_default_system_prompt(style: Style) -> bool {
    !matches!(style, Style::Prompt | Style::Professional)
}

/// Returns the system prompt for a style (Python `get_system_prompt`).
pub fn system_prompt(style: Style) -> &'static str {
    match style {
        Style::Prompt => PROMPT_SYSTEM_PROMPT,
        Style::Professional => PROFESSIONAL_SYSTEM_PROMPT,
        _ => SYSTEM_PROMPT,
    }
}

/// Returns the instruction prompt for a style (Python `get_instruction_prompt`).
pub fn instruction_prompt(style: Style) -> &'static str {
    match style {
        Style::Normal => NORMAL,
        Style::Professional => PROFESSIONAL,
        Style::TranslateEn => TRANSLATE_EN,
        Style::TranslatePl => TRANSLATE_PL,
        Style::ChangeMeaning => CHANGE_MEANING,
        Style::Summary => SUMMARY,
        Style::Prompt => PROMPT,
    }
}

/// Builds the user-message body shared by every provider:
/// `"{instruction}\n\n---\n{text}\n---"`.
pub fn user_message(style: Style, text: &str) -> String {
    format!("{}\n\n---\n{}\n---", instruction_prompt(style), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_key_is_case_insensitive_and_falls_back() {
        assert_eq!(Style::from_key("PROFESSIONAL"), Style::Professional);
        assert_eq!(Style::from_key("  translate_en "), Style::TranslateEn);
        assert_eq!(Style::from_key("nonsense"), Style::Normal);
        assert_eq!(Style::from_key(""), Style::Normal);
    }

    #[test]
    fn system_prompt_routing_matches_python() {
        assert_eq!(system_prompt(Style::Prompt), PROMPT_SYSTEM_PROMPT);
        assert_eq!(system_prompt(Style::Professional), PROFESSIONAL_SYSTEM_PROMPT);
        // every other style uses the default editor system prompt
        assert_eq!(system_prompt(Style::Normal), SYSTEM_PROMPT);
        assert_eq!(system_prompt(Style::Summary), SYSTEM_PROMPT);
        assert_eq!(system_prompt(Style::TranslatePl), SYSTEM_PROMPT);
    }

    #[test]
    fn instruction_prompt_is_distinct_per_style() {
        assert!(instruction_prompt(Style::Normal).starts_with("Rewrite the following text"));
        assert!(instruction_prompt(Style::TranslateEn).contains("INTO ENGLISH"));
        assert!(instruction_prompt(Style::TranslatePl).contains("INTO POLISH"));
        assert!(instruction_prompt(Style::Summary).starts_with("Create a concise summary"));
    }

    #[test]
    fn user_message_wraps_text_in_separators() {
        let msg = user_message(Style::Normal, "Helo wrld");
        assert!(msg.ends_with("\n\n---\nHelo wrld\n---"));
        assert!(msg.starts_with("Rewrite the following text"));
    }

    #[test]
    fn key_roundtrips() {
        for s in [
            Style::Normal,
            Style::Professional,
            Style::TranslateEn,
            Style::TranslatePl,
            Style::ChangeMeaning,
            Style::Summary,
            Style::Prompt,
        ] {
            assert_eq!(Style::from_key(s.as_key()), s);
        }
    }
}
