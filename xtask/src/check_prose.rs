//! Prose linter to detect AI writing patterns in documentation and code comments.
//!
//! This module scans markdown files and Rust doc comments for common AI-generated
//! writing patterns and reports them with suggestions for improvement.

use anyhow::Result;
use ignore::WalkBuilder;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Output format for the prose check results.
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for machine processing
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Unknown format '{}'. Use: text or json", s)),
        }
    }
}

/// Configuration for the prose check command.
#[derive(Debug)]
pub struct CheckProseConfig {
    /// Paths to check (files or directories)
    pub paths: Vec<PathBuf>,
    /// Output format
    pub format: OutputFormat,
    /// Verbose output
    pub verbose: bool,
}

impl Default for CheckProseConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            format: OutputFormat::Text,
            verbose: false,
        }
    }
}

/// Severity level for pattern matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Must be fixed before commit
    Error,
    /// Should be reviewed but may be acceptable
    Warning,
}

/// Category of AI writing pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    TelltaleVerbs,
    TelltaleAdjectives,
    TelltaleNouns,
    HeaderPatterns,
    OpeningPatterns,
    TransitionPatterns,
    HedgingLanguage,
    FillerPhrases,
    BuzzwordPhrases,
    EnthusiasmPatterns,
    RhetoricalPatterns,
    FillerStarters,
    SeekingValidation,
    ClosersSignoffs,
    Anthropomorphization,
    TechCliches,
    VagueIntensifiers,
    Superlatives,
    PairedAdjectives,
    TrailingOff,
    VaguePersonalization,
    PermissionPatterns,
    ReassurancePatterns,
    PromisePatterns,
    InclusivityHedging,
    WordyPhrases,
    WeakStarters,
    WeaselWords,
    MetaCommentary,
    CertaintyMarkers,
    ApologyPatterns,
    TemporalMarkers,
    ComparativeStructures,
}

impl Category {
    /// Human-readable name for the category.
    pub fn display_name(&self) -> &'static str {
        match self {
            Category::TelltaleVerbs => "Telltale Verbs",
            Category::TelltaleAdjectives => "Telltale Adjectives",
            Category::TelltaleNouns => "Telltale Nouns",
            Category::HeaderPatterns => "Header Patterns",
            Category::OpeningPatterns => "Opening Patterns",
            Category::TransitionPatterns => "Transition Patterns",
            Category::HedgingLanguage => "Hedging Language",
            Category::FillerPhrases => "Filler Phrases",
            Category::BuzzwordPhrases => "Buzzword Phrases",
            Category::EnthusiasmPatterns => "Enthusiasm Patterns",
            Category::RhetoricalPatterns => "Rhetorical Patterns",
            Category::FillerStarters => "Filler Starters",
            Category::SeekingValidation => "Seeking Validation",
            Category::ClosersSignoffs => "Closers/Sign-offs",
            Category::Anthropomorphization => "Anthropomorphization",
            Category::TechCliches => "Tech Clichés",
            Category::VagueIntensifiers => "Vague Intensifiers",
            Category::Superlatives => "Superlatives",
            Category::PairedAdjectives => "Paired Adjectives",
            Category::TrailingOff => "Trailing Off",
            Category::VaguePersonalization => "Vague Personalization",
            Category::PermissionPatterns => "Permission Patterns",
            Category::ReassurancePatterns => "Reassurance Patterns",
            Category::PromisePatterns => "Promise Patterns",
            Category::InclusivityHedging => "Inclusivity Hedging",
            Category::WordyPhrases => "Wordy Phrases",
            Category::WeakStarters => "Weak Starters",
            Category::WeaselWords => "Weasel Words",
            Category::MetaCommentary => "Meta-Commentary",
            Category::CertaintyMarkers => "Certainty Markers",
            Category::ApologyPatterns => "Apology Patterns",
            Category::TemporalMarkers => "Temporal Markers",
            Category::ComparativeStructures => "Comparative Structures",
        }
    }
}

/// A pattern to match against text.
pub struct Pattern {
    /// The compiled regex for matching
    pub regex: Regex,
    /// Category of this pattern
    pub category: Category,
    /// Severity level
    pub severity: Severity,
    /// Suggestion for what to use instead
    pub suggestion: &'static str,
    /// The original pattern string (for display)
    pub pattern_text: &'static str,
}

impl Pattern {
    /// Create a new pattern from a regex string.
    pub fn new(
        pattern: &'static str,
        category: Category,
        severity: Severity,
        suggestion: &'static str,
    ) -> Result<Self, regex::Error> {
        // Build case-insensitive regex with word boundaries
        let regex_pattern = format!(r"(?i)\b{}\b", pattern);
        let regex = Regex::new(&regex_pattern)?;
        Ok(Pattern {
            regex,
            category,
            severity,
            suggestion,
            pattern_text: pattern,
        })
    }

    /// Create a new pattern from an exact phrase (escaped for regex).
    pub fn phrase(
        phrase: &'static str,
        category: Category,
        severity: Severity,
        suggestion: &'static str,
    ) -> Result<Self, regex::Error> {
        let escaped = regex::escape(phrase);
        let regex_pattern = format!(r"(?i)\b{}\b", escaped);
        let regex = Regex::new(&regex_pattern)?;
        Ok(Pattern {
            regex,
            category,
            severity,
            suggestion,
            pattern_text: phrase,
        })
    }

    /// Create a pattern that matches at the start of a line/sentence.
    pub fn starter(
        phrase: &'static str,
        category: Category,
        severity: Severity,
        suggestion: &'static str,
    ) -> Result<Self, regex::Error> {
        let escaped = regex::escape(phrase);
        // Match at line start or after sentence-ending punctuation
        let regex_pattern = format!(r"(?im)(^|[.!?]\s+){}", escaped);
        let regex = Regex::new(&regex_pattern)?;
        Ok(Pattern {
            regex,
            category,
            severity,
            suggestion,
            pattern_text: phrase,
        })
    }
}

/// A match found in the text.
#[derive(Debug, Serialize)]
pub struct Match {
    /// Path to the file containing the match
    pub file: PathBuf,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// The matched text
    pub matched_text: String,
    /// The pattern that matched
    pub pattern: String,
    /// Category of the pattern
    pub category: Category,
    /// Severity level
    pub severity: Severity,
    /// Suggestion for improvement
    pub suggestion: String,
}

/// Summary of the prose check results.
#[derive(Debug, Serialize)]
pub struct Summary {
    /// Total number of matches found
    pub total_matches: usize,
    /// Number of files checked
    pub files_checked: usize,
    /// Number of files with matches
    pub files_with_matches: usize,
}

/// Complete results of the prose check.
#[derive(Debug, Serialize)]
pub struct CheckResults {
    /// All matches found
    pub matches: Vec<Match>,
    /// Summary statistics
    pub summary: Summary,
}

/// Build all patterns from the AI writing patterns reference.
pub fn build_patterns() -> Vec<Pattern> {
    let mut patterns = Vec::new();

    // Telltale Verbs
    let verbs = [
        ("delve(?:s|d)?(?:\\s+into)?", "explore, examine, look at"),
        ("navigat(?:e|es|ed|ing)", "work through, handle"),
        ("harness(?:es|ed|ing)?", "use, apply"),
        ("leverag(?:e|es|ed|ing)", "use"),
        ("unlock(?:s|ed|ing)?", "enable, allow"),
        ("embrac(?:e|es|ed|ing)", "adopt, use"),
        ("foster(?:s|ed|ing)?", "encourage, support"),
        ("elevat(?:e|es|ed|ing)", "improve, raise"),
        ("streamlin(?:e|es|ed|ing)", "simplify, speed up"),
        ("illuminat(?:e|es|ed|ing)", "explain, clarify"),
        ("bolster(?:s|ed|ing)?", "support, strengthen"),
        ("orchestrat(?:e|es|ed|ing)", "coordinate, organize"),
        ("underscor(?:e|es|ed|ing)", "emphasize, highlight"),
        ("captivat(?:e|es|ed|ing)", "interest, engage"),
        ("resonat(?:e|es|ed|ing)", "connect, appeal"),
        ("reimagin(?:e|es|ed|ing)", "rethink, redesign"),
        ("supercharg(?:e|es|ed|ing)", "boost, accelerate"),
        ("facilitat(?:e|es|ed|ing)", "help, enable"),
        ("empower(?:s|ed|ing)?", "enable, let"),
        ("spearhead(?:s|ed|ing)?", "lead"),
        ("catalyz(?:e|es|ed|ing)", "trigger, start"),
        ("synergiz(?:e|es|ed|ing)", "combine, work together"),
        ("optimiz(?:e|es|ed|ing)", "improve"),
        ("revolutioniz(?:e|es|ed|ing)", "change, transform"),
    ];
    for (pattern, suggestion) in verbs {
        if let Ok(p) = Pattern::new(
            pattern,
            Category::TelltaleVerbs,
            Severity::Error,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Telltale Adjectives
    let adjectives = [
        ("robust", "strong, reliable"),
        ("seamless(?:ly)?", "smooth, easy"),
        ("comprehensive", "complete, full"),
        ("pivotal", "important, key"),
        ("vibrant", "active, lively"),
        ("meticulous(?:ly)?", "careful, thorough"),
        ("multifaceted", "complex, varied"),
        ("dynamic", "active, changing"),
        ("profound(?:ly)?", "deep, significant"),
        ("bespoke", "custom"),
        ("paramount", "critical, essential"),
        ("cutting-edge", "modern, latest"),
        ("transformative", "significant"),
        ("revolutionary", "new, major"),
        ("game-changing", "significant"),
        ("holistic(?:ally)?", "complete, whole"),
        ("nuanced", "subtle, detailed"),
        ("actionable", "practical, usable"),
        ("scalable", "expandable"),
        ("innovative", "new"),
        ("groundbreaking", "new, first"),
        ("state-of-the-art", "modern, current"),
        ("best-in-class", "leading"),
        ("world-class", "excellent"),
        ("next-generation", "new, upcoming"),
        ("bleeding-edge", "experimental"),
        ("mission-critical", "essential"),
        ("enterprise-grade", "professional"),
        ("battle-tested", "proven"),
    ];
    for (pattern, suggestion) in adjectives {
        if let Ok(p) = Pattern::new(
            pattern,
            Category::TelltaleAdjectives,
            Severity::Error,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Telltale Nouns
    let nouns = [
        ("journey", "process, experience"),
        ("tapestry", "mix, combination"),
        ("realm", "area, field"),
        ("landscape", "situation, field"),
        ("beacon", "guide, example"),
        ("symphony", "coordination, harmony"),
        ("paradigm", "model, approach"),
        ("interplay", "interaction"),
        ("testament", "proof, evidence"),
        ("annals", "history, records"),
        ("ethos", "values, culture"),
        ("synergy", "cooperation"),
        ("ecosystem", "system, environment"),
        ("framework", "structure, system"),
        ("stakeholder(?:s)?", "user, team, person"),
        ("bandwidth", "time, capacity"),
        ("deliverable(?:s)?", "output, result"),
        ("learnings", "lessons"),
        ("cadence", "schedule, rhythm"),
        ("alignment", "agreement"),
        ("deep dive", "analysis, review"),
        ("low-hanging fruit", "easy wins"),
        ("heavy lifting", "hard work"),
        ("north star", "goal, vision"),
    ];
    for (pattern, suggestion) in nouns {
        if let Ok(p) = Pattern::new(
            pattern,
            Category::TelltaleNouns,
            Severity::Error,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Header Patterns
    let headers = [
        ("The Problem", "use descriptive header"),
        ("The Solution", "use descriptive header"),
        ("Why This Matters", "use descriptive header"),
        ("Key Takeaways", "use descriptive header"),
        ("The Bottom Line", "use descriptive header"),
        ("What You'll Learn", "use descriptive header"),
        ("In This Guide", "use descriptive header"),
    ];
    for (phrase, suggestion) in headers {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::HeaderPatterns,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Opening Patterns
    let openers = [
        ("Think of it as", "rephrase directly"),
        ("Whether you're", "rephrase directly"),
        ("Imagine", "rephrase directly"),
        ("Picture this", "rephrase directly"),
        ("Have you ever wondered", "state directly"),
        ("In today's digital age", "omit or be specific"),
        ("In today's fast-paced world", "omit or be specific"),
        ("In today's modern landscape", "omit or be specific"),
        ("In the realm of", "omit"),
        ("When it comes to", "about, for"),
        ("It's no secret that", "omit"),
        ("Let's face it", "omit"),
        ("Here's the thing", "omit"),
        ("At the end of the day", "ultimately, overall"),
        ("The fact of the matter is", "omit"),
        ("It goes without saying", "omit"),
    ];
    for (phrase, suggestion) in openers {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::OpeningPatterns,
            Severity::Error,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Transition Patterns
    let transitions = [
        ("At its core", "fundamentally, essentially"),
        ("Worth noting", "note that, also"),
        ("Simply put", "just say it simply"),
        ("That being said", "however, but"),
        ("From a broader perspective", "more broadly"),
        ("Moving forward", "next, going forward"),
        ("With that in mind", "so, therefore"),
        ("It's important to note", "note:"),
        ("As mentioned earlier", "reference specifically or omit"),
        ("In other words", "rephrase clearly the first time"),
        ("To put it another way", "say it clearly once"),
        ("All things considered", "overall"),
        ("By the same token", "similarly"),
        ("On the flip side", "however, conversely"),
        ("Firstly", "first"),
        ("Secondly", "second"),
        ("Thirdly", "third"),
        ("Furthermore", "also, and"),
        ("Moreover", "also, and"),
        ("Additionally", "also, and"),
        ("In conclusion", "omit or use sparingly"),
        ("To summarize", "omit or use sparingly"),
        ("On the other hand", "however, but"),
        ("In contrast", "however, but"),
    ];
    for (phrase, suggestion) in transitions {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::TransitionPatterns,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Hedging Language
    let hedging = [
        ("generally speaking", "use sparingly"),
        ("tends to", "use sparingly"),
        ("arguably", "use sparingly"),
        ("to some extent", "use sparingly"),
        ("in many cases", "use sparingly"),
        ("more often than not", "use sparingly"),
        ("for the most part", "use sparingly"),
        ("by and large", "use sparingly"),
        ("as a general rule", "use sparingly"),
    ];
    for (phrase, suggestion) in hedging {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::HedgingLanguage,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Filler Phrases
    let fillers = [
        ("It's worth mentioning that", "omit"),
        ("It should be noted that", "omit"),
        ("It's important to understand that", "omit"),
        ("One thing to keep in mind is", "omit"),
        ("The key thing to remember is", "omit"),
        ("What's particularly interesting is", "omit"),
        ("What makes this unique is", "omit"),
        ("This is particularly relevant because", "omit"),
        ("This is especially true when", "omit"),
    ];
    for (phrase, suggestion) in fillers {
        if let Ok(p) = Pattern::phrase(phrase, Category::FillerPhrases, Severity::Error, suggestion)
        {
            patterns.push(p);
        }
    }

    // Buzzword Phrases
    let buzzwords = [
        ("leverage synergies", "work together"),
        ("drive innovation", "create new things"),
        ("move the needle", "make progress"),
        ("circle back", "follow up"),
        ("take it offline", "discuss later"),
        ("boil the ocean", "do too much"),
        ("peel back the onion", "examine closely"),
        ("drink from the firehose", "learn a lot quickly"),
        ("open the kimono", "share information"),
        ("blue sky thinking", "creative ideas"),
        ("thought leadership", "expertise"),
        ("value proposition", "benefit"),
        ("pain points", "problems"),
        ("use case", "example, scenario"),
        ("best practices", "recommendations"),
        ("core competencies", "skills, strengths"),
        ("key differentiator", "advantage"),
        ("value-add", "benefit"),
        ("win-win", "mutual benefit"),
        ("at scale", "widely"),
        ("end-to-end", "complete"),
        ("out of the box", "built-in, default"),
    ];
    for (phrase, suggestion) in buzzwords {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::BuzzwordPhrases,
            Severity::Error,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Enthusiasm Patterns
    let enthusiasm = [
        ("Let's dive in", "omit"),
        ("Let's get started", "omit"),
        ("Let's explore", "omit"),
        ("Let's take a look", "omit"),
        ("Happy coding", "omit"),
        ("Happy building", "omit"),
        ("Enjoy!", "omit"),
        ("Have fun!", "omit"),
        ("This is exciting", "omit"),
        ("Great question", "omit"),
        ("Excellent choice", "omit"),
        ("I'd be happy to", "omit"),
        ("I hope this helps", "omit"),
    ];
    for (phrase, suggestion) in enthusiasm {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::EnthusiasmPatterns,
            Severity::Error,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Rhetorical Patterns
    let rhetorical = [
        ("But what does this mean for you", "omit"),
        ("So what's the catch", "omit"),
        ("Sound familiar", "omit"),
        ("You might be wondering", "omit"),
        ("But wait, there's more", "omit"),
        ("Here's the kicker", "omit"),
        ("Here's the deal", "omit"),
        ("The good news is", "omit"),
        ("The bad news is", "omit"),
        ("Spoiler alert", "omit"),
        ("Plot twist", "omit"),
        ("Fun fact", "omit"),
        ("Pro tip", "use sparingly"),
    ];
    for (phrase, suggestion) in rhetorical {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::RhetoricalPatterns,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Filler Starters
    let starters = [
        ("Basically,", "omit"),
        ("Essentially,", "use sparingly"),
        ("Actually,", "omit"),
        ("Honestly,", "omit"),
        ("To be honest,", "omit"),
        ("Interestingly,", "omit"),
        ("Surprisingly,", "omit"),
        ("Importantly,", "omit"),
        ("Ultimately,", "use sparingly"),
        ("Specifically,", "omit when filler"),
        ("Okay, so", "omit"),
        ("Alright, so", "omit"),
    ];
    for (phrase, suggestion) in starters {
        if let Ok(p) = Pattern::starter(
            phrase,
            Category::FillerStarters,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Seeking Validation
    let validation = [
        ("Right\\?", "omit"),
        ("Make sense\\?", "omit"),
        ("Got it\\?", "omit"),
        ("See\\?", "omit"),
        ("You know\\?", "omit"),
        ("Agreed\\?", "omit"),
    ];
    for (pattern, suggestion) in validation {
        if let Ok(p) = Pattern::new(
            pattern,
            Category::SeekingValidation,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Closers and Sign-offs
    let closers = [
        ("And there you have it", "omit"),
        ("And that's it", "omit"),
        ("And that's a wrap", "omit"),
        ("Voilà", "omit"),
        ("Easy peasy", "omit"),
        ("Piece of cake", "omit"),
        ("Bob's your uncle", "omit"),
        ("Without further ado", "omit"),
    ];
    for (phrase, suggestion) in closers {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::ClosersSignoffs,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Anthropomorphization
    let anthropo = [
        ("The function wants", "rephrase"),
        ("The compiler is happy", "rephrase"),
        ("The server doesn't like", "rephrase"),
        ("Git wants you to", "rephrase"),
        ("The code is trying to", "rephrase"),
        ("is your friend", "rephrase"),
    ];
    for (phrase, suggestion) in anthropo {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::Anthropomorphization,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Tech Clichés
    let tech_cliches = [
        ("under the hood", "internally, implementation"),
        ("behind the scenes", "internally"),
        ("automagically", "automatically"),
        ("nitty-gritty", "details"),
        ("bells and whistles", "features"),
        ("gotchas", "caveats, issues"),
        ("silver bullet", "solution"),
        ("Swiss army knife", "versatile tool"),
        ("one-size-fits-all", "universal"),
        ("plug and play", "ready to use"),
        ("set it and forget it", "automatic"),
        ("batteries included", "complete, full-featured"),
    ];
    for (phrase, suggestion) in tech_cliches {
        if let Ok(p) = Pattern::phrase(phrase, Category::TechCliches, Severity::Warning, suggestion)
        {
            patterns.push(p);
        }
    }

    // Vague Intensifiers
    let intensifiers = [
        ("super easy", "easy"),
        ("super helpful", "helpful"),
        ("really important", "important"),
        ("very important", "important, critical"),
        ("pretty straightforward", "straightforward"),
    ];
    for (phrase, suggestion) in intensifiers {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::VagueIntensifiers,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Superlatives Without Evidence
    let superlatives = [
        ("the best", "a good, an effective"),
        ("the fastest", "fast, quick"),
        ("the most powerful", "powerful"),
        ("top-notch", "good, quality"),
        ("first-class", "good, quality"),
        ("industry-leading", "be specific"),
        ("unparalleled", "be specific"),
        ("unmatched", "be specific"),
        ("second to none", "be specific"),
    ];
    for (phrase, suggestion) in superlatives {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::Superlatives,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Paired/Redundant Adjectives
    let paired = [
        ("quick and easy", "pick one"),
        ("simple and straightforward", "pick one"),
        ("fast and efficient", "pick one"),
        ("powerful and flexible", "pick one"),
        ("clean and elegant", "pick one"),
        ("safe and secure", "pick one"),
        ("complete and comprehensive", "pick one"),
    ];
    for (phrase, suggestion) in paired {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::PairedAdjectives,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Trailing Off
    let trailing = [
        ("and more", "be specific"),
        ("and so on", "be specific"),
        ("and much more", "be specific"),
        ("among others", "be specific"),
        ("to name a few", "be specific"),
        ("the list goes on", "be specific"),
    ];
    for (phrase, suggestion) in trailing {
        if let Ok(p) = Pattern::phrase(phrase, Category::TrailingOff, Severity::Warning, suggestion)
        {
            patterns.push(p);
        }
    }

    // Vague Personalization
    let personalization = [
        ("your use case", "be specific"),
        ("your needs", "be specific"),
        ("your workflow", "be specific"),
        ("your situation", "be specific"),
        ("depending on your requirements", "be specific"),
        ("as needed", "be specific"),
        ("as appropriate", "be specific"),
        ("when necessary", "be specific"),
    ];
    for (phrase, suggestion) in personalization {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::VaguePersonalization,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Permission/Invitation Patterns
    let permission = [
        ("Feel free to", "omit"),
        ("Don't hesitate to", "omit"),
        ("Go ahead and", "omit"),
        ("You're welcome to", "omit"),
        ("Please don't hesitate to reach out", "omit"),
    ];
    for (phrase, suggestion) in permission {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::PermissionPatterns,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Reassurance Patterns
    let reassurance = [
        ("Don't worry", "omit"),
        ("Rest assured", "omit"),
        ("No need to panic", "omit"),
        ("It's okay if", "omit"),
        ("There's no wrong way to", "omit"),
        ("You've got this", "omit"),
    ];
    for (phrase, suggestion) in reassurance {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::ReassurancePatterns,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Promise Patterns
    let promises = [
        ("By the end of this guide, you'll", "omit"),
        ("After reading this, you'll be able to", "omit"),
        ("Once you understand", "omit"),
        ("This will help you", "be specific"),
        ("You'll learn how to", "omit"),
    ];
    for (phrase, suggestion) in promises {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::PromisePatterns,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Inclusivity Hedging
    let inclusivity = [
        ("Whether you're a beginner or an expert", "omit"),
        ("No matter your skill level", "omit"),
        ("Regardless of your experience", "omit"),
        ("For developers of all levels", "omit"),
        ("Even if you've never", "omit"),
    ];
    for (phrase, suggestion) in inclusivity {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::InclusivityHedging,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Wordy Phrases
    let wordy = [
        ("in order to", "to"),
        ("due to the fact that", "because"),
        ("prior to", "before"),
        ("subsequent to", "after"),
        ("in the event that", "if"),
        ("with regard to", "about"),
        ("with respect to", "about"),
        ("in terms of", "omit or rephrase"),
        ("on a daily basis", "daily"),
        ("at this point in time", "now"),
        ("at the present moment", "now"),
        ("for the purpose of", "to, for"),
        ("in the process of", "omit"),
        ("it is important to note that", "just state it"),
        ("the fact that", "that, omit"),
        ("in light of the fact that", "because, since"),
        ("despite the fact that", "although"),
        ("owing to the fact that", "because"),
    ];
    for (phrase, suggestion) in wordy {
        if let Ok(p) = Pattern::phrase(phrase, Category::WordyPhrases, Severity::Error, suggestion)
        {
            patterns.push(p);
        }
    }

    // Weak Sentence Starters
    let weak = [
        ("It should be noted that", "just state it"),
        ("It can be seen that", "omit"),
        ("It goes without saying", "then don't say it"),
        ("It is worth mentioning", "just mention it"),
        ("It is interesting to note", "omit"),
    ];
    for (phrase, suggestion) in weak {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::WeakStarters,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Weasel Words
    let weasel = [
        ("Some people say", "be specific"),
        ("It is believed that", "cite source"),
        ("Many experts think", "cite source"),
        ("Studies show", "cite specific study"),
        ("Research suggests", "cite specific research"),
        ("It is widely accepted", "cite source"),
        ("It is generally known", "cite source"),
        ("Conventional wisdom holds", "cite source"),
    ];
    for (phrase, suggestion) in weasel {
        if let Ok(p) = Pattern::phrase(phrase, Category::WeaselWords, Severity::Warning, suggestion)
        {
            patterns.push(p);
        }
    }

    // Meta-Commentary
    let meta = [
        ("As mentioned earlier", "reference specifically or omit"),
        ("As we discussed", "reference specifically or omit"),
        ("As we'll see later", "omit"),
        ("As I said before", "reference specifically or omit"),
        ("Let me explain", "just explain"),
        ("Allow me to elaborate", "just elaborate"),
        ("I should mention", "just mention it"),
        ("I want to point out", "just point it out"),
        ("It bears repeating", "just repeat it"),
    ];
    for (phrase, suggestion) in meta {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::MetaCommentary,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Certainty Markers
    let certainty = [
        ("Clearly,", "omit"),
        ("Obviously,", "omit"),
        ("It's clear that", "omit"),
        ("It's obvious that", "omit"),
        ("Undoubtedly,", "omit"),
        ("Without a doubt,", "omit"),
        ("Of course,", "use sparingly"),
        ("Naturally,", "use sparingly"),
        ("Needless to say,", "then don't say it"),
    ];
    for (phrase, suggestion) in certainty {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::CertaintyMarkers,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Apology Patterns
    let apology = [
        ("I apologize for any confusion", "omit"),
        ("Sorry for the inconvenience", "omit"),
        ("I apologize if that wasn't clear", "omit"),
        ("My apologies for the oversight", "omit"),
        ("Sorry, I should have mentioned", "omit"),
    ];
    for (phrase, suggestion) in apology {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::ApologyPatterns,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Temporal Markers
    let temporal = [
        ("In recent years", "be specific"),
        ("Nowadays", "be specific"),
        ("In today's world", "omit"),
        ("In the modern era", "omit"),
        ("As of late", "recently"),
        ("Historically speaking", "historically"),
    ];
    for (phrase, suggestion) in temporal {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::TemporalMarkers,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    // Comparative Structures (used sparingly - common pattern but often fine)
    let comparative = [
        ("Unlike X,", "use sparingly"),
        ("Compared to", "use sparingly"),
        ("In contrast to", "use sparingly"),
        ("As opposed to", "use sparingly"),
    ];
    for (phrase, suggestion) in comparative {
        if let Ok(p) = Pattern::phrase(
            phrase,
            Category::ComparativeStructures,
            Severity::Warning,
            suggestion,
        ) {
            patterns.push(p);
        }
    }

    patterns
}

/// A span of text to check, with its source location.
#[derive(Debug)]
pub struct TextSpan {
    /// The text content to check
    pub text: String,
    /// Starting line number (1-indexed)
    pub start_line: usize,
    /// Source file path
    pub file: PathBuf,
}

/// Represents a file and the text spans to check within it.
#[derive(Debug)]
pub struct FileContent {
    /// Path to the file
    #[allow(dead_code)] // Used for debugging/tracing
    pub path: PathBuf,
    /// Text spans to check (may be full file or extracted doc comments)
    pub spans: Vec<TextSpan>,
}

/// Discover files to check in the given paths.
///
/// Finds markdown files and Rust source files, respecting .gitignore.
pub fn discover_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if should_check_file(path) {
                files.push(path.clone());
            }
            continue;
        }

        // Walk directory using ignore crate (respects .gitignore)
        let walker = WalkBuilder::new(path)
            .standard_filters(true) // Respects .gitignore, hidden files, etc.
            .build();

        for entry in walker.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() && should_check_file(entry_path) {
                files.push(entry_path.to_path_buf());
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

/// Check if a file should be scanned for prose patterns.
fn should_check_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        // Check for README, CHANGELOG without extension
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        return file_name.starts_with("README") || file_name.starts_with("CHANGELOG");
    };

    matches!(ext.to_lowercase().as_str(), "md" | "rs")
}

/// Extract text spans from a file for checking.
///
/// For markdown files, returns the full content minus code blocks.
/// For Rust files, extracts doc comments.
pub fn extract_text_spans(path: &PathBuf) -> Result<FileContent> {
    let content = fs::read_to_string(path)?;

    let spans = match path.extension().and_then(|e| e.to_str()) {
        Some("md") => extract_markdown_spans(&content, path),
        Some("rs") => extract_rust_doc_spans(&content, path),
        _ => {
            // For files without extension (README, CHANGELOG), treat as markdown
            extract_markdown_spans(&content, path)
        }
    };

    Ok(FileContent {
        path: path.clone(),
        spans,
    })
}

/// Extract text spans from markdown, skipping code blocks.
fn extract_markdown_spans(content: &str, path: &Path) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut span_start_line = 1;
    let mut in_code_block = false;

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;

        // Check for code block delimiter
        if line.trim_start().starts_with("```") {
            if in_code_block {
                // End of code block
                in_code_block = false;
            } else {
                // Start of code block - save any accumulated text first
                if !current_text.trim().is_empty() {
                    spans.push(TextSpan {
                        text: std::mem::take(&mut current_text),
                        start_line: span_start_line,
                        file: path.to_path_buf(),
                    });
                }
                in_code_block = true;
                current_text.clear();
            }
            continue;
        }

        if !in_code_block {
            if current_text.is_empty() {
                span_start_line = line_num;
            }
            current_text.push_str(line);
            current_text.push('\n');
        }
    }

    // Don't forget the last span
    if !current_text.trim().is_empty() {
        spans.push(TextSpan {
            text: current_text,
            start_line: span_start_line,
            file: path.to_path_buf(),
        });
    }

    spans
}

/// Extract doc comments from Rust source code.
fn extract_rust_doc_spans(content: &str, path: &Path) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    let mut current_doc = String::new();
    let mut doc_start_line = 0;
    let mut in_doc_block = false;

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim_start();

        // Check for doc comments: /// or //!
        let doc_content = if let Some(rest) = trimmed.strip_prefix("///") {
            Some(rest.strip_prefix(' ').unwrap_or(rest))
        } else if let Some(rest) = trimmed.strip_prefix("//!") {
            Some(rest.strip_prefix(' ').unwrap_or(rest))
        } else {
            None
        };

        if let Some(text) = doc_content {
            if !in_doc_block {
                doc_start_line = line_num;
                in_doc_block = true;
            }
            current_doc.push_str(text);
            current_doc.push('\n');
        } else {
            // End of doc block
            if in_doc_block && !current_doc.trim().is_empty() {
                spans.push(TextSpan {
                    text: std::mem::take(&mut current_doc),
                    start_line: doc_start_line,
                    file: path.to_path_buf(),
                });
            }
            in_doc_block = false;
            current_doc.clear();
        }
    }

    // Don't forget the last doc block
    if in_doc_block && !current_doc.trim().is_empty() {
        spans.push(TextSpan {
            text: current_doc,
            start_line: doc_start_line,
            file: path.to_path_buf(),
        });
    }

    spans
}

/// Find matches of patterns in a text span.
pub fn find_matches_in_span(span: &TextSpan, patterns: &[Pattern]) -> Vec<Match> {
    let mut matches = Vec::new();

    for (line_offset, line) in span.text.lines().enumerate() {
        let line_num = span.start_line + line_offset;

        for pattern in patterns {
            for regex_match in pattern.regex.find_iter(line) {
                matches.push(Match {
                    file: span.file.clone(),
                    line: line_num,
                    column: regex_match.start() + 1, // 1-indexed
                    matched_text: regex_match.as_str().to_string(),
                    pattern: pattern.pattern_text.to_string(),
                    category: pattern.category,
                    severity: pattern.severity,
                    suggestion: pattern.suggestion.to_string(),
                });
            }
        }
    }

    matches
}

/// Run the prose linter with the given configuration.
pub fn run(config: CheckProseConfig) -> Result<()> {
    if config.verbose {
        println!("Checking prose in {} path(s)...", config.paths.len());
        for path in &config.paths {
            println!("  - {}", path.display());
        }
    }

    let patterns = build_patterns();
    if config.verbose {
        println!(
            "Loaded {} patterns across {} categories",
            patterns.len(),
            33
        );
    }

    // Discover files to check
    let files = discover_files(&config.paths);
    if config.verbose {
        println!("Found {} files to check", files.len());
    }

    // Process each file and collect matches
    let mut all_matches: Vec<Match> = Vec::new();
    let mut files_checked = 0;
    let mut files_with_matches = 0;

    for file_path in &files {
        match extract_text_spans(file_path) {
            Ok(file_content) => {
                files_checked += 1;
                let mut file_had_matches = false;

                for span in &file_content.spans {
                    let matches = find_matches_in_span(span, &patterns);
                    if !matches.is_empty() {
                        file_had_matches = true;
                        all_matches.extend(matches);
                    }
                }

                if file_had_matches {
                    files_with_matches += 1;
                }
            }
            Err(e) => {
                if config.verbose {
                    eprintln!("Warning: Could not read {}: {}", file_path.display(), e);
                }
            }
        }
    }

    // Build results
    let results = CheckResults {
        matches: all_matches,
        summary: Summary {
            total_matches: 0, // Will be set below
            files_checked,
            files_with_matches,
        },
    };
    let total_matches = results.matches.len();

    // Output results
    match config.format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "matches": results.matches,
                "summary": {
                    "total_matches": total_matches,
                    "files_checked": files_checked,
                    "files_with_matches": files_with_matches
                }
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            for m in &results.matches {
                println!(
                    "{}:{}:{}: [{}] '{}' (suggestion: {})",
                    m.file.display(),
                    m.line,
                    m.column,
                    m.category.display_name(),
                    m.matched_text,
                    m.suggestion
                );
            }
            println!();
            println!(
                "Summary: {} issues found in {} files ({} files checked)",
                total_matches, files_with_matches, files_checked
            );
        }
    }

    // Exit with error if any matches found
    if total_matches > 0 {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_new() {
        let pattern =
            Pattern::new("delve", Category::TelltaleVerbs, Severity::Error, "explore").unwrap();
        assert!(pattern.regex.is_match("delve into"));
        assert!(pattern.regex.is_match("Delve into"));
        assert!(pattern.regex.is_match("DELVE"));
        assert!(!pattern.regex.is_match("delved")); // word boundary should prevent this
    }

    #[test]
    fn test_pattern_phrase() {
        let pattern = Pattern::phrase(
            "Let's dive in",
            Category::EnthusiasmPatterns,
            Severity::Error,
            "omit",
        )
        .unwrap();
        assert!(pattern.regex.is_match("Let's dive in!"));
        assert!(pattern.regex.is_match("let's dive in"));
    }

    #[test]
    fn test_build_patterns_loads_all() {
        let patterns = build_patterns();
        assert!(
            patterns.len() > 200,
            "Expected at least 200 patterns, got {}",
            patterns.len()
        );
    }

    #[test]
    fn test_category_display_name() {
        assert_eq!(Category::TelltaleVerbs.display_name(), "Telltale Verbs");
        assert_eq!(Category::BuzzwordPhrases.display_name(), "Buzzword Phrases");
    }

    #[test]
    fn test_severity_serialization() {
        let error = Severity::Error;
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(json, r#""error""#);
    }

    #[test]
    fn test_verb_conjugations() {
        let patterns = build_patterns();
        let delve_pattern = patterns
            .iter()
            .find(|p| p.pattern_text.contains("delve"))
            .expect("Should have delve pattern");

        assert!(delve_pattern.regex.is_match("delve into"));
        assert!(delve_pattern.regex.is_match("delves into"));
        assert!(delve_pattern.regex.is_match("delved into"));
    }

    #[test]
    fn test_should_check_file() {
        assert!(should_check_file(Path::new("README.md")));
        assert!(should_check_file(Path::new("src/lib.rs")));
        assert!(should_check_file(Path::new("docs/guide.MD")));
        assert!(should_check_file(Path::new("README")));
        assert!(should_check_file(Path::new("CHANGELOG")));
        assert!(!should_check_file(Path::new("main.py")));
        assert!(!should_check_file(Path::new("image.png")));
    }

    #[test]
    fn test_extract_markdown_spans_skips_code_blocks() {
        let content = r#"# Header

This is prose that should be checked.

```rust
fn delve() {
    // This should be skipped
}
```

More prose after the code block.
"#;
        let spans = extract_markdown_spans(content, Path::new("test.md"));

        // Should have 2 spans (before and after code block)
        assert_eq!(spans.len(), 2);

        // First span should include header and prose
        assert!(spans[0].text.contains("Header"));
        assert!(spans[0].text.contains("prose that should be checked"));
        assert!(!spans[0].text.contains("fn delve"));
        assert_eq!(spans[0].start_line, 1);

        // Second span should include prose after code block
        // Starts at line 10 (empty line after code block ends)
        assert!(spans[1].text.contains("More prose after"));
        assert_eq!(spans[1].start_line, 10);
    }

    #[test]
    fn test_extract_markdown_spans_handles_nested_code_markers() {
        let content = r#"Some text

```
code block
```

More text
"#;
        let spans = extract_markdown_spans(content, Path::new("test.md"));
        assert_eq!(spans.len(), 2);
        assert!(!spans[0].text.contains("code block"));
    }

    #[test]
    fn test_extract_rust_doc_spans() {
        let content = r#"//! Module-level docs.
//! This is also module docs.

/// Function docs.
/// More function docs.
fn my_func() {}

// Regular comment (not doc)
fn other_func() {}

/// Another doc comment.
fn third_func() {}
"#;
        let spans = extract_rust_doc_spans(content, Path::new("test.rs"));

        // Should have 3 doc blocks
        assert_eq!(spans.len(), 3);

        // Module docs
        assert!(spans[0].text.contains("Module-level docs"));
        assert_eq!(spans[0].start_line, 1);

        // Function docs
        assert!(spans[1].text.contains("Function docs"));
        assert!(spans[1].text.contains("More function docs"));
        assert_eq!(spans[1].start_line, 4);

        // Another doc comment
        assert!(spans[2].text.contains("Another doc comment"));
        assert_eq!(spans[2].start_line, 11);
    }

    #[test]
    fn test_find_matches_in_span() {
        let span = TextSpan {
            text: "Let's delve into this topic.\nThis is robust and seamless.".to_string(),
            start_line: 10,
            file: PathBuf::from("test.md"),
        };

        let patterns = build_patterns();
        let matches = find_matches_in_span(&span, &patterns);

        // Should find at least delve, robust, seamless
        assert!(
            matches.len() >= 3,
            "Expected at least 3 matches, got {}",
            matches.len()
        );

        // Check that delve match is on line 10
        let delve_match = matches
            .iter()
            .find(|m| m.matched_text.to_lowercase().contains("delve"));
        assert!(delve_match.is_some(), "Should find 'delve' match");
        assert_eq!(delve_match.unwrap().line, 10);

        // Check that robust/seamless are on line 11
        let robust_match = matches
            .iter()
            .find(|m| m.matched_text.to_lowercase() == "robust");
        assert!(robust_match.is_some(), "Should find 'robust' match");
        assert_eq!(robust_match.unwrap().line, 11);
    }

    #[test]
    fn test_find_matches_reports_correct_columns() {
        let span = TextSpan {
            text: "The robust system works.".to_string(),
            start_line: 1,
            file: PathBuf::from("test.md"),
        };

        let patterns = build_patterns();
        let matches = find_matches_in_span(&span, &patterns);

        let robust_match = matches
            .iter()
            .find(|m| m.matched_text.to_lowercase() == "robust");
        assert!(robust_match.is_some());
        // "robust" starts at column 5 (1-indexed)
        assert_eq!(robust_match.unwrap().column, 5);
    }

    #[test]
    fn test_match_serialization() {
        let m = Match {
            file: PathBuf::from("test.md"),
            line: 10,
            column: 5,
            matched_text: "delve".to_string(),
            pattern: "delve".to_string(),
            category: Category::TelltaleVerbs,
            severity: Severity::Error,
            suggestion: "explore".to_string(),
        };

        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"line\":10"));
        assert!(json.contains("\"severity\":\"error\""));
        assert!(json.contains("\"category\":\"telltale_verbs\""));
    }

    #[test]
    fn test_output_format_parsing() {
        assert!(matches!(
            "text".parse::<OutputFormat>(),
            Ok(OutputFormat::Text)
        ));
        assert!(matches!(
            "TEXT".parse::<OutputFormat>(),
            Ok(OutputFormat::Text)
        ));
        assert!(matches!(
            "json".parse::<OutputFormat>(),
            Ok(OutputFormat::Json)
        ));
        assert!(matches!(
            "JSON".parse::<OutputFormat>(),
            Ok(OutputFormat::Json)
        ));
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_check_prose_config_default() {
        let config = CheckProseConfig::default();
        assert_eq!(config.paths, vec![PathBuf::from(".")]);
        assert!(matches!(config.format, OutputFormat::Text));
        assert!(!config.verbose);
    }

    #[test]
    fn test_discover_files_on_xtask_directory() {
        // Verify that discover_files can scan the xtask src directory
        // This test runs from the xtask directory, so use src/
        let files = discover_files(&[PathBuf::from("src")]);

        // Should find some Rust files in src/
        let rs_count = files
            .iter()
            .filter(|f| f.extension().and_then(|e| e.to_str()) == Some("rs"))
            .count();
        assert!(rs_count > 0, "Should find some Rust files in xtask/src");

        // Verify we found this file specifically
        assert!(
            files.iter().any(|f| f.ends_with("check_prose.rs")),
            "Should find check_prose.rs"
        );
    }

    #[test]
    fn test_pattern_starter() {
        let pattern = Pattern::starter(
            "Basically,",
            Category::FillerStarters,
            Severity::Warning,
            "omit",
        )
        .unwrap();

        // Should match at start of line
        assert!(pattern.regex.is_match("Basically, this is the point"));

        // Should match after sentence ending
        assert!(pattern
            .regex
            .is_match("End of sentence. Basically, new point"));
    }

    #[test]
    fn test_extract_text_spans_handles_file_not_found() {
        let result = extract_text_spans(&PathBuf::from("nonexistent.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_case_insensitive_matching() {
        let patterns = build_patterns();
        let span = TextSpan {
            text: "LEVERAGE this ROBUST solution".to_string(),
            start_line: 1,
            file: PathBuf::from("test.md"),
        };

        let matches = find_matches_in_span(&span, &patterns);
        assert!(matches
            .iter()
            .any(|m| m.matched_text.to_uppercase() == "LEVERAGE"));
        assert!(matches
            .iter()
            .any(|m| m.matched_text.to_uppercase() == "ROBUST"));
    }

    #[test]
    fn test_summary_serialization() {
        let summary = Summary {
            total_matches: 5,
            files_checked: 10,
            files_with_matches: 3,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_matches\":5"));
        assert!(json.contains("\"files_checked\":10"));
        assert!(json.contains("\"files_with_matches\":3"));
    }
}
