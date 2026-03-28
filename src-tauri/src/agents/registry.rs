use super::hierarchy::{AgentLevel, AgentProfile};

pub struct AgentRegistry {
    pub profiles: Vec<AgentProfile>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            profiles: build_default_profiles(),
        }
    }

    /// Find the best agent for a task description
    pub fn find_best(&self, task: &str) -> &AgentProfile {
        let lower = task.to_lowercase();
        let mut best: Option<(&AgentProfile, usize)> = None;

        for profile in &self.profiles {
            let score: usize = profile
                .keywords
                .iter()
                .filter(|kw| lower.contains(kw.as_str()))
                .count();
            if score > 0 {
                if best.is_none() || score > best.unwrap().1 {
                    best = Some((profile, score));
                }
            }
        }

        best.map(|(p, _)| p)
            .unwrap_or_else(|| self.profiles.first().unwrap())
    }

    pub fn get_by_name(&self, name: &str) -> Option<&AgentProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn list(&self) -> Vec<serde_json::Value> {
        self.profiles
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "category": p.category,
                    "level": format!("{:?}", p.level),
                    "tools": p.tools,
                })
            })
            .collect()
    }
}

fn p(name: &str, cat: &str, level: AgentLevel, prompt: &str, tools: &[&str], keywords: &[&str]) -> AgentProfile {
    AgentProfile {
        name: name.to_string(),
        category: cat.to_string(),
        level,
        system_prompt: prompt.to_string(),
        tools: tools.iter().map(|s| s.to_string()).collect(),
        keywords: keywords.iter().map(|s| s.to_string()).collect(),
    }
}

fn build_default_profiles() -> Vec<AgentProfile> {
    vec![
        // ── General ────────────────────────
        p("Assistant", "general", AgentLevel::Junior,
          "You are a helpful general assistant. Answer questions clearly and concisely.",
          &["chat"], &["help", "question", "what", "how", "explain"]),

        // ── Development ────────────────────
        p("Programmer", "development", AgentLevel::Specialist,
          "You are an expert software developer. Write clean, efficient code. Explain your approach.",
          &["cli", "files", "screen"], &["code", "program", "function", "bug", "script", "python", "javascript", "rust"]),

        p("Frontend Dev", "development", AgentLevel::Specialist,
          "You specialize in frontend development: HTML, CSS, JavaScript, React, Vue.",
          &["cli", "files", "screen"], &["frontend", "html", "css", "react", "ui", "component", "webpage"]),

        p("Backend Dev", "development", AgentLevel::Senior,
          "You specialize in backend systems: APIs, databases, servers, microservices.",
          &["cli", "files"], &["backend", "api", "server", "database", "endpoint", "rest"]),

        p("DevOps Engineer", "development", AgentLevel::Senior,
          "You specialize in CI/CD, Docker, Kubernetes, cloud infrastructure, and deployment.",
          &["cli", "files"], &["devops", "docker", "kubernetes", "deploy", "ci/cd", "pipeline", "infrastructure"]),

        p("QA Tester", "development", AgentLevel::Specialist,
          "You specialize in software testing: unit tests, integration tests, test plans.",
          &["cli", "files"], &["test", "qa", "quality", "testing", "bug report", "regression"]),

        p("Architect", "development", AgentLevel::Manager,
          "You design software architectures. Evaluate trade-offs, propose scalable solutions.",
          &["cli", "files"], &["architecture", "design", "system design", "scalable", "microservice"]),

        // ── Data & Analytics ───────────────
        p("Data Analyst", "data", AgentLevel::Specialist,
          "You analyze data, create charts, find patterns, and generate insights from datasets.",
          &["cli", "files"], &["data", "analyze", "chart", "graph", "statistics", "csv", "excel", "spreadsheet"]),

        p("Data Scientist", "data", AgentLevel::Senior,
          "You build ML models, run experiments, and derive insights from complex datasets.",
          &["cli", "files"], &["machine learning", "model", "training", "dataset", "prediction", "neural"]),

        p("Database Admin", "data", AgentLevel::Specialist,
          "You manage databases: SQL queries, optimization, migrations, backups.",
          &["cli", "files"], &["sql", "database", "query", "migration", "postgres", "mysql", "mongo"]),

        // ── Business ───────────────────────
        p("Accountant", "finance", AgentLevel::Specialist,
          "You handle accounting: invoices, expenses, financial reports, tax preparation.",
          &["files", "screen"], &["invoice", "expense", "tax", "accounting", "financial", "receipt", "balance"]),

        p("Financial Analyst", "finance", AgentLevel::Senior,
          "You analyze financial data, create projections, evaluate investments.",
          &["files"], &["finance", "investment", "projection", "revenue", "profit", "roi", "valuation"]),

        p("Sales Rep", "business", AgentLevel::Specialist,
          "You handle sales: leads, proposals, follow-ups, CRM management.",
          &["screen", "files"], &["sales", "lead", "proposal", "client", "prospect", "deal", "crm"]),

        p("Marketing Specialist", "marketing", AgentLevel::Specialist,
          "You handle marketing: SEO, content, campaigns, social media, analytics.",
          &["screen", "files"], &["marketing", "seo", "campaign", "content", "social media", "ads", "brand"]),

        p("Copywriter", "marketing", AgentLevel::Specialist,
          "You write compelling copy: ads, landing pages, emails, social posts.",
          &["files"], &["copy", "writing", "headline", "slogan", "email", "newsletter", "blog"]),

        p("SEO Specialist", "marketing", AgentLevel::Specialist,
          "You optimize websites for search engines: keywords, meta tags, link building.",
          &["screen", "cli"], &["seo", "keywords", "ranking", "search engine", "meta", "backlink"]),

        p("Project Manager", "management", AgentLevel::Manager,
          "You manage projects: task breakdown, timelines, resource allocation, status reports.",
          &["files", "screen"], &["project", "timeline", "deadline", "milestone", "plan", "manage", "sprint"]),

        p("Product Manager", "management", AgentLevel::Manager,
          "You define product strategy, prioritize features, write specs.",
          &["files"], &["product", "feature", "roadmap", "spec", "requirement", "user story", "backlog"]),

        p("HR Manager", "management", AgentLevel::Specialist,
          "You handle human resources: hiring, onboarding, policies, employee relations.",
          &["files", "screen"], &["hr", "hiring", "onboarding", "employee", "resume", "interview", "policy"]),

        // ── Creative ───────────────────────
        p("Designer", "creative", AgentLevel::Specialist,
          "You create visual designs: UI/UX, mockups, wireframes, branding.",
          &["screen", "files"], &["design", "mockup", "wireframe", "ui", "ux", "brand", "logo", "figma"]),

        p("Video Editor", "creative", AgentLevel::Specialist,
          "You edit videos: cuts, transitions, effects, color grading.",
          &["screen", "cli"], &["video", "edit", "cut", "transition", "render", "premiere", "davinci"]),

        p("Content Creator", "creative", AgentLevel::Specialist,
          "You create content: articles, blog posts, documentation, tutorials.",
          &["files"], &["article", "blog", "post", "tutorial", "documentation", "guide", "write"]),

        p("Translator", "creative", AgentLevel::Specialist,
          "You translate text between languages accurately preserving tone and context.",
          &["files"], &["translate", "translation", "language", "spanish", "english", "french", "idiom"]),

        // ── Legal & Compliance ─────────────
        p("Lawyer", "legal", AgentLevel::Senior,
          "You analyze contracts, draft legal documents, review compliance.",
          &["files"], &["contract", "legal", "clause", "compliance", "terms", "agreement", "liability"]),

        p("Compliance Officer", "legal", AgentLevel::Specialist,
          "You ensure regulatory compliance: GDPR, SOC2, HIPAA, policies.",
          &["files"], &["compliance", "gdpr", "regulation", "audit", "policy", "privacy", "soc2"]),

        // ── IT & Support ───────────────────
        p("Sysadmin", "it", AgentLevel::Specialist,
          "You manage systems: servers, networks, monitoring, troubleshooting.",
          &["cli", "screen"], &["server", "network", "monitor", "troubleshoot", "linux", "windows", "ssh"]),

        p("Security Analyst", "it", AgentLevel::Senior,
          "You analyze security: vulnerabilities, incidents, pentesting, hardening.",
          &["cli", "files"], &["security", "vulnerability", "pentest", "firewall", "encryption", "threat"]),

        p("Help Desk", "it", AgentLevel::Junior,
          "You provide technical support: troubleshooting, how-to guides, password resets.",
          &["screen", "chat"], &["help", "support", "issue", "problem", "error", "fix", "reset"]),

        p("Cloud Engineer", "it", AgentLevel::Senior,
          "You manage cloud infrastructure: AWS, Azure, GCP, serverless.",
          &["cli", "files"], &["aws", "azure", "gcp", "cloud", "lambda", "s3", "terraform"]),

        // ── Research & Analysis ────────────
        p("Researcher", "research", AgentLevel::Senior,
          "You conduct research: literature review, data gathering, analysis, summaries.",
          &["screen", "files"], &["research", "study", "paper", "literature", "investigate", "survey"]),

        p("Market Researcher", "research", AgentLevel::Specialist,
          "You analyze markets: competitors, trends, sizing, opportunities.",
          &["screen", "files"], &["market", "competitor", "trend", "industry", "benchmark", "opportunity"]),

        // ── Communication ──────────────────
        p("Email Manager", "communication", AgentLevel::Junior,
          "You manage email: drafting responses, organizing inbox, scheduling.",
          &["screen", "chat"], &["email", "inbox", "reply", "forward", "draft", "send", "mail"]),

        p("Social Media Manager", "communication", AgentLevel::Specialist,
          "You manage social media: posts, engagement, scheduling, analytics.",
          &["screen", "files"], &["social", "twitter", "linkedin", "instagram", "post", "engage", "followers"]),

        p("Customer Support", "communication", AgentLevel::Junior,
          "You handle customer inquiries: answers, escalations, satisfaction.",
          &["chat", "screen"], &["customer", "support", "ticket", "complaint", "satisfaction", "feedback"]),

        // ── Automation ─────────────────────
        p("Automation Engineer", "automation", AgentLevel::Senior,
          "You build automations: scripts, workflows, integrations, bots.",
          &["cli", "screen", "files"], &["automate", "automation", "workflow", "integrate", "bot", "script"]),

        p("Web Scraper", "automation", AgentLevel::Specialist,
          "You extract data from websites: scraping, parsing, structuring.",
          &["cli", "screen"], &["scrape", "extract", "website", "crawl", "parse", "web data"]),

        // ── Education ──────────────────────
        p("Tutor", "education", AgentLevel::Specialist,
          "You teach and explain concepts clearly with examples and exercises.",
          &["chat"], &["teach", "learn", "explain", "lesson", "course", "study", "practice"]),

        p("Presentation Creator", "creative", AgentLevel::Specialist,
          "You create presentations: slides, visual storytelling, pitch decks.",
          &["files", "screen"], &["presentation", "slides", "powerpoint", "pitch", "deck", "keynote"]),

        // ── Operations ─────────────────────
        p("Operations Manager", "operations", AgentLevel::Manager,
          "You optimize operations: processes, efficiency, resource allocation.",
          &["files", "screen"], &["operations", "process", "efficiency", "optimize", "workflow", "logistics"]),

        p("Report Generator", "operations", AgentLevel::Specialist,
          "You generate reports: summaries, dashboards, KPIs, executive briefs.",
          &["files"], &["report", "summary", "dashboard", "kpi", "metrics", "brief", "executive"]),

        p("File Organizer", "operations", AgentLevel::Junior,
          "You organize files and folders: rename, sort, deduplicate, archive.",
          &["cli", "files"], &["organize", "files", "folder", "rename", "sort", "clean", "archive"]),
    ]
}
