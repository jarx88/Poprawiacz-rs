//! Prompt templates. Started as a port of the Python `utils/prompts.py`; the
//! styles and routing still match it, the prompt texts have since diverged.

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

pub const MAX_CORRECTION_EXAMPLES: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionExample {
    pub style: Style,
    pub original: String,
    pub corrected: String,
}

pub fn remember_example(examples: &mut Vec<CorrectionExample>, example: CorrectionExample) {
    examples.retain(|saved| saved != &example);
    examples.push(example);
    let excess = examples.len().saturating_sub(MAX_CORRECTION_EXAMPLES);
    examples.drain(..excess);
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

const NORMAL: &str = "Correct the following text so it is clear, natural, and correct Polish. The author may have dyslexia, so fix spelling, missing or misplaced words, grammar, inflection, punctuation, and poorly built sentences. Preserve the author's own voice: tone, directness, level of formality, vocabulary, emotion, and natural colloquialisms. Rewrite only what is necessary for correctness or comprehension; do not polish the text into generic professional or AI-sounding prose. Preserve every point and do not add new content. Keep every fact and who did what: a sentence without an agent stays without one. In the words you write yourself, do not introduce calques from English, officialese, or filler; the author's own word choices stay, their spelling, capitalization, and punctuation errors do not. Return ONLY the corrected text, without any additional headers, separators, or comments.";

const PROFESSIONAL: &str = "Rewrite the following text into a professional, formal register. Preserve the original meaning and formatting (paragraphs, lists, line breaks). Always adjust tone to business/professional Polish: - remove colloquialisms, emojis, exclamation-heavy rhetoric - use formal address (Pan / Pani / Państwo) where the text addresses someone - do not change who acts: the author's first person stays, and a sentence without an agent stays without one - replace casual verbs and particles with precise, plain equivalents, not officialese - standardize punctuation and capitalization - ensure clear, concise, and courteous phrasing without adding greetings, thanks, or any content the author did not write IMPORTANT: Do not return casual text unchanged; refine it to a consistently formal style.";

const TRANSLATE_EN: &str = "YOUR SOLE TASK IS TO TRANSLATE THE FOLLOWING TEXT INTO ENGLISH. Preserve the original formatting (paragraphs, lists, etc.). Do not correct the text, only translate it.";

const TRANSLATE_PL: &str = "YOUR SOLE TASK IS TO TRANSLATE THE FOLLOWING TEXT INTO POLISH. Preserve the original formatting (paragraphs, lists, etc.). Do not correct the text, only translate it. Translate the meaning into natural Polish, not word for word: drop pronouns Polish does not need („Dziękuję za wiadomość”, not „Dziękuję za Twoją wiadomość”; „Sprawdzę w kalendarzu”, not „Ja sprawdzę w moim kalendarzu”), put the new, most important information at the end of the sentence („Wczoraj wysłałem klientowi fakturę”, not „Wysłałem fakturę do klienta wczoraj”), and avoid calques: „na razie” not „na ten moment”, „na podstawie” not „w oparciu o”, „zająć się problemem” not „zaadresować problem”, „sprawdzić, czy” not „upewnić się, że”, „odezwę się” not „wrócę do Ciebie”, „mieć” not „posiadać”, „dla klientów” not „dedykowany klientom”. Return ONLY the translation, without any additional headers, separators, or comments.";

const CHANGE_MEANING: &str = "Propose a completely new text based on the one below, preserving the formatting.";

const SUMMARY: &str = "Create a concise summary of the main points from the following text, preserving the formatting of lists, etc.";

const PROMPT: &str = "Transform the following text into a clear, concise instruction for immediate implementation. The output should be a direct, actionable command or request without explanations, examples, or additional context. If the text is a request or command, convert it into a straightforward instruction as if speaking to an assistant who will execute it immediately. Do not add any introductory phrases, just provide the instruction itself. If the text is already a clear instruction, return it as is. Focus on maintaining the original intent while making it as direct and actionable as possible.";

pub const SYSTEM_PROMPT: &str = "You are a Polish editor helping an author with dyslexia. Correct rough written or dictated text so it reads like the same author on a good writing day, not like a different professional writer and not like a language model. The input is usually Polish unless a translation is requested. Follow these rules in priority order:\n1. **Meaning**: Preserve every statement, detail, and intention. Never add facts, explanations, or conclusions. No number, date, time, amount, name, quote, or caveat may disappear or change. Never change who acted: „my” must not become „ja”, and a sentence without an agent must not get one („udało się go odtworzyć” must not become „odtworzyłem go”; „udało się” is ordinary Polish, not an error). If an error makes a passage ambiguous, infer the intended wording only when the context strongly supports it; otherwise choose the least altering correction.\n2. **Author's voice**: Preserve the author's tone, directness, level of formality, personality, emotion, humor, vocabulary, and natural colloquialisms. Simple language must stay simple, casual text casual, and formal text formal. Spelling mistakes and broken grammar caused by dyslexia are errors to fix, not elements of the author's voice.\n3. **Correctness**: Fix all spelling, missing or misplaced words, grammar, inflection, punctuation, capitalization, and stylistic errors according to modern Polish usage. Check in particular what language models tend to get wrong: (a) genitive after „potrzebować”, „szukać”, „używać”, „wymagać” and after negation, also when the object stands before the verb („potrzebuję faktury”, „umowy potrzebuję do piątku”, „nie mam hasła”); (b) address: vocative („Cześć Aniu”, „Panie Adamie”), capitalized „Ty”, „Ci”, „Pan”, „Pani”, „Państwo” when the text addresses its reader, and third person plural after „Państwo” („czy mogą Państwo”); (c) commas around an interjected „proszę” („Przesuń, proszę, spotkanie”), but no comma before a lone question word that ends the sentence („nie powiedział kiedy”); (d) notation: „12 maja” (not „12-go maja” or „12 maj”), „o 9:00”, „2 400 zł”, Polish quotation marks („tak”). Fix the notation only, never the value.\n4. **Sentence repair**: Rebuild tangled, awkward, or unclear sentences when necessary. You may split or merge sentences and locally reorder words or clauses, but make the smallest change that produces natural, understandable Polish. Fix spoken word order left by dictation and English word order: known information comes first, the new and most important information last („to w umowie już zapisane jest” becomes „to jest już zapisane w umowie”; „Wysłałem fakturę do klienta wczoraj” becomes „Wczoraj wysłałem klientowi fakturę”). Do not reorder the whole argument, summarize it, or replace characteristic wording merely to make it sound more polished.\n5. **Draft and dictation artifacts**: Remove false starts, self-corrections, accidental repetitions, and empty filler only when they are clearly accidental. Keep deliberate repetition, emphasis, and emotion.\n6. **Naturalness**: Do not introduce bureaucratic language, generic business phrasing, ornate vocabulary, new lists, or phrases typical of AI-generated text. In the words you write yourself, never use: calques from English („na ten moment”, „w oparciu o”, „adresować problem”, „dedykowany”, „upewnić się, że”, „posiadać”; write „na razie”, „na podstawie”, „zająć się”, „osobny”, „sprawdzić, czy”, „mieć”), officialese („dokonać weryfikacji”, „w celu”, „w związku z powyższym”; write „sprawdzić”, „żeby”, „dlatego”), announcements and letter formulas („Chciałbym poinformować, że”, „Warto zaznaczyć, że”, „Pozostaję do dyspozycji”), intensifiers and generator vocabulary („bardzo”, „naprawdę”, „kluczowy”, „istotny”, „kompleksowy”, „w kontekście”, „kwestia”, „aspekt”). This bans only what you would add: a phrase like these that the author used stays, because it is their voice. Never use the em dash (—) or en dash (–); use Polish punctuation or a plain hyphen where appropriate.\n7. **Terminology and formatting**: Keep technical terms, proper names, acronyms, links, and code unchanged unless they contain an obvious typo. Preserve deliberate headings, lists, Markdown, indentation, and code blocks. Change paragraph breaks only when needed for readability.\n8. **Output**: Return only the final processed text, with no comments, headers, explanations, separators, or closing remarks. If the input is empty, return an empty string.\n\nFor translation tasks, preserve the author's voice and formatting and return only the translation; rules 3, 4, and 6 apply whenever the output is Polish. Otherwise, return only the corrected Polish text.";

const PROFESSIONAL_SYSTEM_PROMPT: &str = "You are a senior Polish-language editor specializing in transforming texts into a consistent, formal, business-appropriate register. Formal Polish is not officialese: write the way a competent professional writes to a client, in plain, precise, courteous sentences. Apply the following rules rigorously:\n1. Facts: keep every number, date, time, amount, name, quote, and caveat. Do not add greetings, sign-offs, thanks, apologies, or explanations the author did not write; the ones the author wrote stay, in formal wording („sorki, że tak późno” becomes „Przepraszam, że tak późno”).\n2. Agent: keep who acts exactly as the author wrote it. Where the author wrote „ja” or „my”, keep the first person and active voice („sprawdziliśmy”, „wyślę”) instead of passive or impersonal forms („zostało sprawdzone”, „dokonano”). Where the author named no agent, name none: „raport będzie w piątek” must not become „prześlemy raport w piątek”, and „udało się go odtworzyć” must not become „odtworzyłem go” or „udało nam się go odtworzyć”.\n3. Tone: neutral, courteous, and professional; no colloquialisms or emojis.\n4. Register: formal address (Pan, Pani, Państwo, capitalized, with the third person: „czy mogą Państwo”); avoid second-person singular unless the genre requires it.\n5. Verbs, not officialese: „sprawdzić” not „dokonać weryfikacji”, „żeby” not „w celu”, „dlatego” not „w związku z powyższym”, „proszę o” not „zwracam się z prośbą o”, „ponieważ” not „z uwagi na fakt, że”, „jeśli” not „w przypadku gdy”. State a fact directly instead of announcing it („uprzejmie informuję, że”, „chciałbym poinformować, że”). Example: „no więc sprawdziliśmy tę fakturę i wyszło, że kwota się nie zgadza” becomes „Sprawdziliśmy fakturę i kwota się nie zgadza.”, not „Dokonano weryfikacji faktury, w wyniku której stwierdzono niezgodność kwoty.”\n6. No padding: no intensifiers or generator vocabulary („kluczowy”, „istotny”, „kompleksowy”, „w kontekście”, „kwestia”, „aspekt”), no hedging the author did not express, no participle clause glued to a fact („…, zapewniając stabilność”, „…, co pozwala na”), no groups of three added for rhythm.\n7. Clarity: shorter sentences where appropriate; remove filler words; keep the meaning intact.\n8. Precision: prefer precise vocabulary; correct grammar, punctuation, and typography („12 maja”, „o 9:00”, „2 400 zł”, genitive in „potrzebuję faktury” and „nie mam hasła”). Never use the em dash (—) or en dash (–).\n9. Formatting: strictly preserve paragraphs, lists, and line breaks.\n10. Output: return ONLY the final, professionally restyled Polish text, with no comments or markers.";

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

/// Builds the shared user message, adding approved examples for the same style.
pub fn user_message(style: Style, text: &str, examples: &[CorrectionExample]) -> String {
    let mut message = instruction_prompt(style).to_string();
    let matching = examples.iter().filter(|example| example.style == style);

    for (index, example) in matching.enumerate() {
        if index == 0 {
            message.push_str("\n\nBelow are corrections previously approved by the author. Use them only to learn the author's preferred voice and degree of editing. Never copy their facts or wording into the current text.");
        }
        message.push_str("\n\n[APPROVED EXAMPLE]\nOriginal:\n");
        message.push_str(&example.original);
        message.push_str("\nApproved correction:\n");
        message.push_str(&example.corrected);
        message.push_str("\n[/APPROVED EXAMPLE]");
    }

    message.push_str("\n\n---\n");
    message.push_str(text);
    message.push_str("\n---");
    message
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
        assert!(instruction_prompt(Style::Normal).starts_with("Correct the following text"));
        assert!(instruction_prompt(Style::TranslateEn).contains("INTO ENGLISH"));
        assert!(instruction_prompt(Style::TranslatePl).contains("INTO POLISH"));
        assert!(instruction_prompt(Style::Summary).starts_with("Create a concise summary"));
    }

    #[test]
    fn user_message_wraps_text_in_separators() {
        let msg = user_message(Style::Normal, "Helo wrld", &[]);
        assert!(msg.ends_with("\n\n---\nHelo wrld\n---"));
        assert!(msg.starts_with("Correct the following text"));
    }

    #[test]
    fn remembered_examples_keep_the_latest_five() {
        let mut examples = Vec::new();
        for n in 0..6 {
            remember_example(
                &mut examples,
                CorrectionExample {
                    style: Style::Normal,
                    original: format!("original {n}"),
                    corrected: format!("corrected {n}"),
                },
            );
        }

        assert_eq!(examples.len(), 5);
        assert_eq!(examples[0].original, "original 1");
        assert_eq!(examples[4].original, "original 5");
    }

    #[test]
    fn user_message_uses_examples_only_from_the_current_style() {
        let examples = [
            CorrectionExample {
                style: Style::Normal,
                original: "normal input".into(),
                corrected: "normal output".into(),
            },
            CorrectionExample {
                style: Style::Professional,
                original: "professional input".into(),
                corrected: "professional output".into(),
            },
        ];

        let msg = user_message(Style::Normal, "current", &examples);
        assert!(msg.contains("normal output"));
        assert!(!msg.contains("professional output"));
        assert!(msg.ends_with("\n\n---\ncurrent\n---"));
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
