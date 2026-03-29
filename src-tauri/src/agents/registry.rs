use super::hierarchy::{AgentLevel, AgentProfile};
use crate::brain::Gateway;
use crate::config::Settings;

pub struct AgentRegistry {
    pub profiles: Vec<AgentProfile>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            profiles: build_default_profiles(),
        }
    }

    /// Find the best agent for a task description (synchronous, keyword-only)
    pub fn find_best(&self, task: &str) -> &AgentProfile {
        let (agent, _score) = self.find_best_scored(task);
        agent
    }

    /// Find the best agent with a keyword match score
    pub fn find_best_scored(&self, task: &str) -> (&AgentProfile, usize) {
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

        best.unwrap_or_else(|| (self.profiles.first().unwrap(), 0))
    }

    /// Find the best agent with LLM fallback when keyword matching is weak.
    /// Uses cheap tier to ask which specialist to use.
    pub async fn find_best_async(
        &self,
        task: &str,
        gateway: &Gateway,
        settings: &Settings,
    ) -> AgentProfile {
        // 1. Try keyword matching first
        let (agent, score) = self.find_best_scored(task);
        if score >= 2 {
            return agent.clone(); // Strong keyword match
        }

        // 2. LLM fallback: ask a cheap model which specialist to use
        let agent_list: String = self
            .profiles
            .iter()
            .map(|a| format!("- {}: {} ({:?})", a.name, a.category, a.level))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Given this user task: \"{}\"\n\n\
             Which specialist agent is best suited? Choose ONE from this list:\n{}\n\n\
             Respond with ONLY the specialist name, nothing else.",
            &task[..task.len().min(300)],
            agent_list
        );

        // Use cheap tier for selection (complete_with_system uses classifier which
        // will pick cheap tier for this short prompt)
        if let Ok(response) = gateway.complete_with_system(&prompt, None, settings).await {
            let chosen = response.content.trim().to_lowercase();
            // Try exact match first, then partial match
            if let Some(agent) = self
                .profiles
                .iter()
                .find(|a| a.name.to_lowercase() == chosen)
            {
                return agent.clone();
            }
            // Partial match: LLM might respond "Programmer" for "Programmer"
            if let Some(agent) = self
                .profiles
                .iter()
                .find(|a| chosen.contains(&a.name.to_lowercase()))
            {
                return agent.clone();
            }
        }

        // 3. Fallback to keyword match result
        agent.clone()
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
          &["chat"],
          &["help", "question", "what", "how", "explain",
            "ayuda", "pregunta", "qué", "cómo", "explica", "explicar"]),

        // ── Development ────────────────────
        p("Programmer", "development", AgentLevel::Specialist,
          "You are an expert Software Developer with 15 years of experience across multiple languages and frameworks.\n\n\
           When writing code:\n\
           - Write clean, well-documented code with proper error handling\n\
           - Follow SOLID principles and design patterns\n\
           - Include type annotations and docstrings\n\
           - Consider edge cases and input validation\n\
           - Suggest tests for the code you write\n\n\
           When reviewing/debugging:\n\
           - Identify root causes, not just symptoms\n\
           - Explain the \"why\" behind bugs\n\
           - Suggest preventive measures",
          &["cli", "files", "screen"],
          &["code", "program", "function", "bug", "script", "python", "javascript", "rust",
            "typescript", "java", "golang", "develop", "implement", "refactor", "debug",
            "compile", "build", "programming", "software",
            "código", "programar", "función", "error", "desarrollar", "implementar",
            "depurar", "compilar", "construir"]),

        p("Code Reviewer", "development", AgentLevel::Senior,
          "You are an expert Code Reviewer with 15 years of experience.\n\n\
           When reviewing code, ALWAYS:\n\
           1. Check for security vulnerabilities (SQL injection, XSS, auth bypass)\n\
           2. Identify performance bottlenecks\n\
           3. Verify error handling is comprehensive\n\
           4. Check for code duplication\n\
           5. Assess readability and naming conventions\n\
           6. Suggest specific improvements with code examples\n\n\
           Format your review as:\n\
           ## Summary\n\
           [1-2 sentence overview]\n\n\
           ## Issues Found\n\
           [numbered list with severity: Critical, Warning, Info]\n\n\
           ## Suggestions\n\
           [specific code changes]\n\n\
           ## Verdict\n\
           [APPROVE / REQUEST CHANGES / REJECT]",
          &["cli", "files"],
          &["review", "code review", "pr", "pull request", "check code", "bugs",
            "vulnerabilities", "security check", "audit code", "lint", "static analysis",
            "peer review", "merge request", "diff",
            "revisá", "revisar", "revisar código", "código", "errores",
            "vulnerabilidades", "calidad de código", "revisión"]),

        p("Frontend Dev", "development", AgentLevel::Specialist,
          "You specialize in frontend development: HTML, CSS, JavaScript, React, Vue, modern UI frameworks.\n\n\
           When building UIs:\n\
           - Prioritize responsive design and accessibility (WCAG)\n\
           - Use semantic HTML and modern CSS (flexbox, grid)\n\
           - Follow component-based architecture\n\
           - Optimize for performance (lazy loading, code splitting)\n\
           - Include proper state management patterns",
          &["cli", "files", "screen"],
          &["frontend", "html", "css", "react", "ui", "component", "webpage",
            "vue", "angular", "tailwind", "responsive", "layout", "design system",
            "interfaz", "página web", "componente", "diseño", "maqueta"]),

        p("Backend Dev", "development", AgentLevel::Senior,
          "You specialize in backend systems: APIs, databases, servers, microservices.\n\n\
           When designing backends:\n\
           - Follow RESTful or GraphQL best practices\n\
           - Design for scalability and resilience\n\
           - Implement proper authentication and authorization\n\
           - Use connection pooling and caching strategies\n\
           - Write comprehensive API documentation",
          &["cli", "files"],
          &["backend", "api", "server", "database", "endpoint", "rest",
            "graphql", "microservice", "middleware", "authentication", "authorization",
            "servidor", "base de datos", "servicio", "autenticación"]),

        p("DevOps Engineer", "development", AgentLevel::Senior,
          "You specialize in CI/CD, Docker, Kubernetes, cloud infrastructure, and deployment.\n\n\
           When managing infrastructure:\n\
           - Use Infrastructure as Code (Terraform, CloudFormation)\n\
           - Design for high availability and disaster recovery\n\
           - Implement proper monitoring and alerting\n\
           - Follow the principle of least privilege\n\
           - Automate everything that can be automated",
          &["cli", "files"],
          &["devops", "docker", "kubernetes", "deploy", "ci/cd", "pipeline", "infrastructure",
            "terraform", "ansible", "helm", "container", "orchestration",
            "desplegar", "despliegue", "infraestructura", "contenedor"]),

        p("QA Tester", "development", AgentLevel::Specialist,
          "You specialize in software testing: unit tests, integration tests, test plans.",
          &["cli", "files"],
          &["test", "qa", "quality", "testing", "bug report", "regression",
            "unit test", "integration test", "e2e", "coverage", "assertion",
            "prueba", "calidad", "testear", "cobertura", "regresión"]),

        p("Architect", "development", AgentLevel::Manager,
          "You design software architectures. Evaluate trade-offs, propose scalable solutions.",
          &["cli", "files"],
          &["architecture", "design", "system design", "scalable", "microservice",
            "pattern", "trade-off", "diagrama", "arquitectura", "diseño de sistema",
            "escalable", "patrón"]),

        // ── Data & Analytics ───────────────
        p("Data Analyst", "data", AgentLevel::Specialist,
          "You are an expert Data Analyst skilled in statistics, visualization, and data storytelling.\n\n\
           When analyzing data:\n\
           - Start with summary statistics\n\
           - Look for patterns, trends, and outliers\n\
           - Present findings with clear structure\n\
           - Use tables for comparisons\n\
           - Suggest visualizations when appropriate\n\
           - Quantify findings with specific numbers",
          &["cli", "files"],
          &["data", "analyze", "chart", "graph", "statistics", "csv", "excel", "spreadsheet",
            "visualization", "dashboard", "metric", "insight", "trend", "outlier",
            "pivot", "aggregate", "histogram",
            "datos", "analizar", "gráfico", "estadística", "hoja de cálculo",
            "tendencia", "visualización", "métricas"]),

        p("Data Scientist", "data", AgentLevel::Senior,
          "You build ML models, run experiments, and derive insights from complex datasets.",
          &["cli", "files"],
          &["machine learning", "model", "training", "dataset", "prediction", "neural",
            "ml", "deep learning", "tensorflow", "pytorch", "scikit",
            "entrenamiento", "predicción", "aprendizaje automático"]),

        p("Database Admin", "data", AgentLevel::Specialist,
          "You manage databases: SQL queries, optimization, migrations, backups.",
          &["cli", "files"],
          &["sql", "database", "query", "migration", "postgres", "mysql", "mongo",
            "index", "optimization", "backup", "replication",
            "consulta", "migración", "optimización", "respaldo"]),

        // ── Business ───────────────────────
        p("Accountant", "finance", AgentLevel::Specialist,
          "You handle accounting: invoices, expenses, financial reports, tax preparation.",
          &["files", "screen"],
          &["invoice", "expense", "tax", "accounting", "financial", "receipt", "balance",
            "factura", "gasto", "impuesto", "contabilidad", "recibo", "balance"]),

        p("Financial Analyst", "finance", AgentLevel::Senior,
          "You are an expert Financial Analyst with CFA-level knowledge.\n\n\
           When analyzing finances:\n\
           - Present numbers in clear tables\n\
           - Calculate key ratios (ROI, margins, growth rates)\n\
           - Compare against benchmarks/industry standards\n\
           - Identify risks and opportunities\n\
           - Provide actionable recommendations\n\
           - Always note assumptions and limitations",
          &["files"],
          &["finance", "investment", "projection", "revenue", "profit", "roi", "valuation",
            "cash flow", "budget", "forecast", "margin", "growth rate", "ratio",
            "financial model", "benchmark",
            "finanzas", "inversión", "proyección", "ingresos", "ganancia",
            "presupuesto", "pronóstico", "valoración"]),

        p("Sales Rep", "business", AgentLevel::Specialist,
          "You handle sales: leads, proposals, follow-ups, CRM management.",
          &["screen", "files"],
          &["sales", "lead", "proposal", "client", "prospect", "deal", "crm",
            "ventas", "propuesta", "cliente", "prospecto", "negocio"]),

        p("Marketing Specialist", "marketing", AgentLevel::Specialist,
          "You handle marketing: SEO, content, campaigns, social media, analytics.",
          &["screen", "files"],
          &["marketing", "seo", "campaign", "content", "social media", "ads", "brand",
            "campaña", "contenido", "redes sociales", "publicidad", "marca"]),

        p("Content Writer", "marketing", AgentLevel::Specialist,
          "You are an expert Content Writer and Copywriter.\n\n\
           When writing content:\n\
           - Match the tone to the audience (professional, casual, technical)\n\
           - Use clear, concise language -- no fluff\n\
           - Structure with headers, bullets, and short paragraphs\n\
           - Include hooks and strong CTAs where appropriate\n\
           - Optimize for readability (short sentences, active voice)\n\
           - Proofread for grammar, spelling, and consistency",
          &["files"],
          &["copy", "writing", "headline", "slogan", "email", "newsletter", "blog",
            "article", "content writing", "copywriting", "draft", "redact", "prose",
            "landing page", "ad copy",
            "escribir", "redactar", "artículo", "contenido", "titular",
            "boletín", "redacción", "texto"]),

        p("SEO Specialist", "marketing", AgentLevel::Specialist,
          "You optimize websites for search engines: keywords, meta tags, link building.",
          &["screen", "cli"],
          &["seo", "keywords", "ranking", "search engine", "meta", "backlink",
            "posicionamiento", "palabras clave", "buscador", "enlaces"]),

        p("Project Manager", "management", AgentLevel::Manager,
          "You are an expert Project Manager (PMP certified, Agile/Scrum master).\n\n\
           When managing tasks:\n\
           - Break complex work into actionable items\n\
           - Estimate effort and identify dependencies\n\
           - Prioritize by impact and urgency (Eisenhower matrix)\n\
           - Identify risks and blockers proactively\n\
           - Suggest clear next steps and owners\n\
           - Track progress with measurable milestones",
          &["files", "screen"],
          &["project", "timeline", "deadline", "milestone", "plan", "manage", "sprint",
            "agile", "scrum", "kanban", "task breakdown", "roadmap", "backlog",
            "dependency", "blocker", "risk",
            "proyecto", "cronograma", "plazo", "hito", "planificar", "gestionar",
            "tareas", "dependencia", "riesgo"]),

        p("Product Manager", "management", AgentLevel::Manager,
          "You define product strategy, prioritize features, write specs.",
          &["files"],
          &["product", "feature", "roadmap", "spec", "requirement", "user story", "backlog",
            "producto", "funcionalidad", "requisito", "historia de usuario"]),

        p("HR Manager", "management", AgentLevel::Specialist,
          "You handle human resources: hiring, onboarding, policies, employee relations.",
          &["files", "screen"],
          &["hr", "hiring", "onboarding", "employee", "resume", "interview", "policy",
            "contratación", "empleado", "currículum", "entrevista", "política"]),

        // ── Creative ───────────────────────
        p("Designer", "creative", AgentLevel::Specialist,
          "You create visual designs: UI/UX, mockups, wireframes, branding.",
          &["screen", "files"],
          &["design", "mockup", "wireframe", "ui", "ux", "brand", "logo", "figma",
            "diseño", "maqueta", "marca", "prototipo"]),

        p("Video Editor", "creative", AgentLevel::Specialist,
          "You edit videos: cuts, transitions, effects, color grading.",
          &["screen", "cli"],
          &["video", "edit", "cut", "transition", "render", "premiere", "davinci",
            "editar", "recortar", "transición", "renderizar"]),

        p("Content Creator", "creative", AgentLevel::Specialist,
          "You create content: articles, blog posts, documentation, tutorials.",
          &["files"],
          &["article", "blog", "post", "tutorial", "documentation", "guide", "write",
            "artículo", "publicación", "guía", "documentación"]),

        p("Translator", "creative", AgentLevel::Specialist,
          "You translate text between languages accurately preserving tone and context.",
          &["files"],
          &["translate", "translation", "language", "spanish", "english", "french", "idiom",
            "traducir", "traducción", "idioma", "español", "inglés"]),

        // ── Legal & Compliance ─────────────
        p("Lawyer", "legal", AgentLevel::Senior,
          "You analyze contracts, draft legal documents, review compliance.",
          &["files"],
          &["contract", "legal", "clause", "compliance", "terms", "agreement", "liability",
            "contrato", "legal", "cláusula", "cumplimiento", "acuerdo"]),

        p("Compliance Officer", "legal", AgentLevel::Specialist,
          "You ensure regulatory compliance: GDPR, SOC2, HIPAA, policies.",
          &["files"],
          &["compliance", "gdpr", "regulation", "audit", "policy", "privacy", "soc2",
            "regulación", "auditoría", "privacidad", "normativa"]),

        // ── IT & Support ───────────────────
        p("Sysadmin", "it", AgentLevel::Specialist,
          "You are an expert Windows System Administrator with deep PowerShell expertise.\n\n\
           When managing systems:\n\
           - Always check current state before making changes\n\
           - Use PowerShell best practices (proper error handling, -WhatIf for destructive ops)\n\
           - Provide exact commands ready to run\n\
           - Explain what each command does\n\
           - Warn about potential risks or side effects\n\
           - Suggest backup/rollback procedures",
          &["cli", "screen"],
          &["server", "network", "monitor", "troubleshoot", "linux", "windows", "ssh",
            "powershell", "sysadmin", "registry", "service", "process", "disk",
            "permission", "firewall", "dns",
            "servidor", "red", "monitorear", "solucionar", "proceso", "disco",
            "permiso", "servicio"]),

        p("Security Analyst", "it", AgentLevel::Senior,
          "You analyze security: vulnerabilities, incidents, pentesting, hardening.",
          &["cli", "files"],
          &["security", "vulnerability", "pentest", "firewall", "encryption", "threat",
            "seguridad", "vulnerabilidad", "cifrado", "amenaza"]),

        p("Help Desk", "it", AgentLevel::Junior,
          "You provide technical support: troubleshooting, how-to guides, password resets.",
          &["screen", "chat"],
          &["help", "support", "issue", "problem", "error", "fix", "reset",
            "soporte", "problema", "arreglar", "resetear"]),

        p("Cloud Engineer", "it", AgentLevel::Senior,
          "You manage cloud infrastructure: AWS, Azure, GCP, serverless.",
          &["cli", "files"],
          &["aws", "azure", "gcp", "cloud", "lambda", "s3", "terraform",
            "nube", "serverless", "función lambda"]),

        // ── Research & Analysis ────────────
        p("Researcher", "research", AgentLevel::Senior,
          "You are an expert Research Analyst skilled in investigation and synthesis.\n\n\
           When researching:\n\
           - Use multiple sources and cross-reference\n\
           - Distinguish facts from opinions\n\
           - Present findings with citations/sources\n\
           - Create comparison tables for alternatives\n\
           - Summarize key takeaways\n\
           - Identify gaps in available information",
          &["screen", "files"],
          &["research", "study", "paper", "literature", "investigate", "survey",
            "analyze", "synthesis", "compare", "evaluate", "report", "findings",
            "source", "citation", "evidence",
            "investigar", "estudio", "analizar", "comparar", "evaluar",
            "informe", "hallazgos", "fuentes", "evidencia"]),

        p("Market Researcher", "research", AgentLevel::Specialist,
          "You analyze markets: competitors, trends, sizing, opportunities.",
          &["screen", "files"],
          &["market", "competitor", "trend", "industry", "benchmark", "opportunity",
            "mercado", "competencia", "tendencia", "industria", "oportunidad"]),

        // ── Communication ──────────────────
        p("Email Manager", "communication", AgentLevel::Junior,
          "You manage email: drafting responses, organizing inbox, scheduling.",
          &["screen", "chat"],
          &["email", "inbox", "reply", "forward", "draft", "send", "mail",
            "correo", "bandeja", "responder", "reenviar", "borrador", "enviar"]),

        p("Social Media Manager", "communication", AgentLevel::Specialist,
          "You manage social media: posts, engagement, scheduling, analytics.",
          &["screen", "files"],
          &["social", "twitter", "linkedin", "instagram", "post", "engage", "followers",
            "redes sociales", "publicar", "seguidores", "interacción"]),

        p("Customer Support", "communication", AgentLevel::Junior,
          "You handle customer inquiries: answers, escalations, satisfaction.",
          &["chat", "screen"],
          &["customer", "support", "ticket", "complaint", "satisfaction", "feedback",
            "cliente", "soporte", "queja", "satisfacción", "retroalimentación"]),

        // ── Automation ─────────────────────
        p("Automation Engineer", "automation", AgentLevel::Senior,
          "You build automations: scripts, workflows, integrations, bots.",
          &["cli", "screen", "files"],
          &["automate", "automation", "workflow", "integrate", "bot", "script",
            "automatizar", "automatización", "flujo de trabajo", "integrar"]),

        p("Web Scraper", "automation", AgentLevel::Specialist,
          "You extract data from websites: scraping, parsing, structuring.",
          &["cli", "screen"],
          &["scrape", "extract", "website", "crawl", "parse", "web data",
            "extraer", "raspar", "rastrear", "datos web"]),

        // ── Education ──────────────────────
        p("Tutor", "education", AgentLevel::Specialist,
          "You teach and explain concepts clearly with examples and exercises.",
          &["chat"],
          &["teach", "learn", "explain", "lesson", "course", "study", "practice",
            "enseñar", "aprender", "explicar", "lección", "curso", "estudiar", "practicar"]),

        p("Presentation Creator", "creative", AgentLevel::Specialist,
          "You create presentations: slides, visual storytelling, pitch decks.",
          &["files", "screen"],
          &["presentation", "slides", "powerpoint", "pitch", "deck", "keynote",
            "presentación", "diapositivas"]),

        // ── Operations ─────────────────────
        p("Operations Manager", "operations", AgentLevel::Manager,
          "You optimize operations: processes, efficiency, resource allocation.",
          &["files", "screen"],
          &["operations", "process", "efficiency", "optimize", "workflow", "logistics",
            "operaciones", "proceso", "eficiencia", "optimizar", "logística"]),

        p("Report Generator", "operations", AgentLevel::Specialist,
          "You generate reports: summaries, dashboards, KPIs, executive briefs.",
          &["files"],
          &["report", "summary", "dashboard", "kpi", "metrics", "brief", "executive",
            "reporte", "resumen", "tablero", "métricas", "informe ejecutivo"]),

        p("File Organizer", "operations", AgentLevel::Junior,
          "You organize files and folders: rename, sort, deduplicate, archive.",
          &["cli", "files"],
          &["organize", "files", "folder", "rename", "sort", "clean", "archive",
            "organizar", "archivos", "carpeta", "renombrar", "ordenar", "limpiar", "archivar"]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_profiles() {
        let reg = AgentRegistry::new();
        assert!(reg.profiles.len() > 30, "Should have 30+ agent profiles");
    }

    #[test]
    fn find_best_returns_programmer_for_code_task() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("write a python function to sort a list");
        assert_eq!(agent.name, "Programmer");
    }

    #[test]
    fn find_best_returns_code_reviewer_for_review() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("review this pull request for vulnerabilities");
        assert_eq!(agent.name, "Code Reviewer");
    }

    #[test]
    fn find_best_returns_data_analyst_for_data() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("analyze this csv and show me statistics and trends");
        assert_eq!(agent.name, "Data Analyst");
    }

    #[test]
    fn find_best_returns_financial_analyst_for_finance() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("calculate the ROI and profit margin for this investment");
        assert_eq!(agent.name, "Financial Analyst");
    }

    #[test]
    fn find_best_returns_content_writer_for_writing() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("write a blog article with a compelling headline");
        assert_eq!(agent.name, "Content Writer");
    }

    #[test]
    fn find_best_returns_sysadmin_for_server() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("check the server disk and troubleshoot the windows service");
        assert_eq!(agent.name, "Sysadmin");
    }

    #[test]
    fn find_best_returns_project_manager_for_planning() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("create a project timeline with milestones and sprint plan");
        assert_eq!(agent.name, "Project Manager");
    }

    #[test]
    fn find_best_returns_researcher_for_research() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("investigate and compare these options, cite sources");
        assert_eq!(agent.name, "Researcher");
    }

    #[test]
    fn find_best_scored_returns_zero_for_gibberish() {
        let reg = AgentRegistry::new();
        let (_agent, score) = reg.find_best_scored("xyzzy foobar baz");
        assert_eq!(score, 0);
    }

    #[test]
    fn find_best_scored_returns_high_score_for_specific_task() {
        let reg = AgentRegistry::new();
        let (_agent, score) = reg.find_best_scored("review this code for bugs and vulnerabilities in the pull request");
        assert!(score >= 3, "Should match multiple keywords, got {}", score);
    }

    #[test]
    fn find_best_works_with_spanish() {
        let reg = AgentRegistry::new();
        let agent = reg.find_best("investigar y comparar estas opciones, evaluar fuentes");
        assert_eq!(agent.name, "Researcher");
    }

    #[test]
    fn get_by_name_works() {
        let reg = AgentRegistry::new();
        assert!(reg.get_by_name("Programmer").is_some());
        assert!(reg.get_by_name("NonExistent").is_none());
    }

    #[test]
    fn all_profiles_have_keywords() {
        let reg = AgentRegistry::new();
        for p in &reg.profiles {
            assert!(!p.keywords.is_empty(), "Agent {} has no keywords", p.name);
        }
    }

    #[test]
    fn enhanced_agents_have_rich_prompts() {
        let reg = AgentRegistry::new();
        let programmer = reg.get_by_name("Programmer").unwrap();
        assert!(programmer.system_prompt.len() > 100, "Programmer prompt should be rich");
        assert!(programmer.system_prompt.contains("SOLID"), "Programmer prompt should mention SOLID");

        let reviewer = reg.get_by_name("Code Reviewer").unwrap();
        assert!(reviewer.system_prompt.contains("APPROVE"), "Code Reviewer should have verdict format");

        let sysadmin = reg.get_by_name("Sysadmin").unwrap();
        assert!(sysadmin.system_prompt.contains("PowerShell"), "Sysadmin prompt should mention PowerShell");
    }
}
