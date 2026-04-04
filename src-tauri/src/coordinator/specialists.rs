use crate::coordinator::types::AgentLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecialistProfile {
    pub id: String,
    pub name: String,
    pub category: SpecialistCategory,
    pub level: AgentLevel,
    pub description: String,
    pub system_prompt: String,
    pub default_tools: Vec<String>,
    pub default_model_tier: String,
    pub icon: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpecialistCategory {
    SoftwareDev,
    DesignCreative,
    BusinessFinance,
    MarketingGrowth,
    DataAnalytics,
    Operations,
    Sales,
    Legal,
    Research,
}

impl SpecialistCategory {
    pub fn label(&self) -> &'static str {
        match self {
            SpecialistCategory::SoftwareDev => "Software Development",
            SpecialistCategory::DesignCreative => "Design & Creative",
            SpecialistCategory::BusinessFinance => "Business & Finance",
            SpecialistCategory::MarketingGrowth => "Marketing & Growth",
            SpecialistCategory::DataAnalytics => "Data & Analytics",
            SpecialistCategory::Operations => "Operations",
            SpecialistCategory::Sales => "Sales",
            SpecialistCategory::Legal => "Legal",
            SpecialistCategory::Research => "Research",
        }
    }
}

pub struct SpecialistRegistry {
    profiles: HashMap<String, SpecialistProfile>,
}

impl SpecialistRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            profiles: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    pub fn get(&self, id: &str) -> Option<&SpecialistProfile> {
        self.profiles.get(id)
    }

    pub fn exists(&self, id: &str) -> bool {
        self.profiles.contains_key(id)
    }

    pub fn list(&self) -> Vec<SpecialistProfile> {
        let mut values = self.profiles.values().cloned().collect::<Vec<_>>();
        values.sort_by(|left, right| left.name.cmp(&right.name));
        values
    }

    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = self
            .list()
            .into_iter()
            .map(|profile| {
                format!(
                    "- {} ({:?}): {}. Tools: {}",
                    profile.id,
                    profile.level,
                    profile.description,
                    profile.default_tools.join(", ")
                )
            })
            .collect::<Vec<_>>();
        lines.sort();
        lines
    }

    fn add(&mut self, profile: SpecialistProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    fn register_defaults(&mut self) {
        let defs = vec![
            specialist(
                "software_architect",
                "Software Architect",
                SpecialistCategory::SoftwareDev,
                AgentLevel::Senior,
                "Designs system architecture, APIs, and technical decisions",
                "You are a senior software architect. Produce implementation-ready architecture, explain tradeoffs clearly, and prefer maintainable systems.",
                &["read_file", "write_file", "search_files", "bash"],
                "premium",
                "cpu",
            ),
            specialist(
                "backend_dev",
                "Backend Developer",
                SpecialistCategory::SoftwareDev,
                AgentLevel::Senior,
                "Writes backend code, APIs, database queries, and server logic",
                "You are a backend developer. Write production-ready backend code, handle edge cases, and leave systems better than you found them.",
                &["read_file", "write_file", "edit_file", "search_files", "bash"],
                "standard",
                "server",
            ),
            specialist(
                "frontend_dev",
                "Frontend Developer",
                SpecialistCategory::SoftwareDev,
                AgentLevel::Senior,
                "Builds UI components, pages, and user interfaces",
                "You are a frontend developer specializing in React and TypeScript. Build accessible, responsive, polished interfaces that respect the product's visual language.",
                &["read_file", "write_file", "edit_file", "search_files", "bash"],
                "standard",
                "layout",
            ),
            specialist(
                "devops_engineer",
                "DevOps Engineer",
                SpecialistCategory::SoftwareDev,
                AgentLevel::Specialist,
                "Manages deployments, CI/CD, infrastructure, and monitoring",
                "You are a DevOps engineer. Optimize delivery pipelines, reliability, reproducibility, and operational visibility.",
                &["bash", "read_file", "write_file", "edit_file", "search_files"],
                "standard",
                "cloud",
            ),
            specialist(
                "qa_tester",
                "QA Tester",
                SpecialistCategory::SoftwareDev,
                AgentLevel::Specialist,
                "Writes tests, finds bugs, and verifies acceptance criteria",
                "You are a QA engineer. Think in edge cases, regressions, and user journeys. Prefer reproducible findings and concrete test plans.",
                &["read_file", "write_file", "bash", "search_files"],
                "standard",
                "bug",
            ),
            specialist(
                "db_admin",
                "Database Administrator",
                SpecialistCategory::SoftwareDev,
                AgentLevel::Specialist,
                "Designs schemas, writes queries, optimizes database performance",
                "You are a database administrator. Design clean schemas, safe migrations, and efficient queries with clear reasoning.",
                &["bash", "read_file", "write_file"],
                "standard",
                "database",
            ),
            specialist(
                "ui_designer",
                "UI Designer",
                SpecialistCategory::DesignCreative,
                AgentLevel::Senior,
                "Creates interface directions, design systems, and polished visual specs",
                "You are a UI designer. Produce deliberate, memorable, usable interface concepts with strong hierarchy and refined detail.",
                &["read_file", "write_file", "web_search", "screenshot"],
                "premium",
                "palette",
            ),
            specialist(
                "ux_researcher",
                "UX Researcher",
                SpecialistCategory::DesignCreative,
                AgentLevel::Specialist,
                "Researches user behavior, friction, and product opportunities",
                "You are a UX researcher. Ground recommendations in evidence, uncover friction, and translate findings into practical design direction.",
                &["web_search", "web_browse", "read_file", "write_file"],
                "standard",
                "scan-search",
            ),
            specialist(
                "copywriter",
                "Copywriter",
                SpecialistCategory::DesignCreative,
                AgentLevel::Specialist,
                "Writes compelling copy for campaigns, products, and landing pages",
                "You are a copywriter. Write concise, persuasive copy with a sharp sense of audience, tone, and conversion intent.",
                &["read_file", "write_file", "web_search"],
                "standard",
                "pen-tool",
            ),
            specialist(
                "brand_strategist",
                "Brand Strategist",
                SpecialistCategory::DesignCreative,
                AgentLevel::Senior,
                "Shapes positioning, messaging architecture, and brand differentiation",
                "You are a brand strategist. Build coherent positioning, messaging frameworks, and differentiated creative direction.",
                &["web_search", "web_browse", "read_file", "write_file"],
                "premium",
                "sparkles",
            ),
            specialist(
                "financial_analyst",
                "Financial Analyst",
                SpecialistCategory::BusinessFinance,
                AgentLevel::Senior,
                "Analyzes financial data, creates projections, and writes reports",
                "You are a financial analyst. Be precise with numbers, assumptions, and scenario analysis. Explain business implications clearly.",
                &["read_file", "write_file", "bash", "web_search", "web_browse"],
                "premium",
                "trending-up",
            ),
            specialist(
                "accountant",
                "Accountant",
                SpecialistCategory::BusinessFinance,
                AgentLevel::Specialist,
                "Handles bookkeeping, reconciliations, and structured financial records",
                "You are an accountant. Keep records tidy, auditable, and clearly categorized.",
                &["read_file", "write_file", "bash"],
                "standard",
                "calculator",
            ),
            specialist(
                "business_strategist",
                "Business Strategist",
                SpecialistCategory::BusinessFinance,
                AgentLevel::Senior,
                "Develops strategy, positioning, and opportunity analysis",
                "You are a business strategist. Use clear frameworks, challenge assumptions, and land on actionable recommendations.",
                &["web_search", "web_browse", "read_file", "write_file"],
                "premium",
                "target",
            ),
            specialist(
                "investment_analyst",
                "Investment Analyst",
                SpecialistCategory::BusinessFinance,
                AgentLevel::Senior,
                "Evaluates deals, projections, and diligence materials",
                "You are an investment analyst. Evaluate risk, upside, and assumptions rigorously, and communicate confidence levels.",
                &["web_search", "web_browse", "read_file", "write_file"],
                "premium",
                "line-chart",
            ),
            specialist(
                "seo_specialist",
                "SEO Specialist",
                SpecialistCategory::MarketingGrowth,
                AgentLevel::Specialist,
                "Optimizes content for search, intent alignment, and discoverability",
                "You are an SEO specialist. Think in search intent, structure, keyword prioritization, and measurable content improvements.",
                &["web_search", "web_browse", "write_file", "read_file"],
                "cheap",
                "globe",
            ),
            specialist(
                "content_marketer",
                "Content Marketer",
                SpecialistCategory::MarketingGrowth,
                AgentLevel::Specialist,
                "Creates articles, campaign copy, and content calendars",
                "You are a content marketer. Produce useful, engaging, channel-aware content with strong narrative clarity.",
                &["web_search", "write_file", "read_file"],
                "standard",
                "megaphone",
            ),
            specialist(
                "social_media_manager",
                "Social Media Manager",
                SpecialistCategory::MarketingGrowth,
                AgentLevel::Specialist,
                "Builds social campaigns, calendars, and channel strategy",
                "You are a social media manager. Adapt messaging by platform, optimize for engagement, and keep campaigns coherent.",
                &["web_search", "write_file", "read_file"],
                "standard",
                "radio",
            ),
            specialist(
                "growth_hacker",
                "Growth Hacker",
                SpecialistCategory::MarketingGrowth,
                AgentLevel::Senior,
                "Designs experiments, funnels, and growth loops",
                "You are a growth operator. Focus on leverage, experimentation speed, funnel clarity, and measurable wins.",
                &["read_file", "write_file", "web_search", "bash"],
                "premium",
                "rocket",
            ),
            specialist(
                "data_analyst",
                "Data Analyst",
                SpecialistCategory::DataAnalytics,
                AgentLevel::Specialist,
                "Analyzes data, creates spreadsheets, and extracts insights",
                "You are a data analyst. Cleanly summarize trends, quantify results, and present conclusions stakeholders can trust.",
                &["read_file", "write_file", "bash", "search_files"],
                "standard",
                "bar-chart-2",
            ),
            specialist(
                "data_engineer",
                "Data Engineer",
                SpecialistCategory::DataAnalytics,
                AgentLevel::Specialist,
                "Builds ETL flows, data contracts, and warehouse pipelines",
                "You are a data engineer. Design dependable data movement and clear ownership boundaries.",
                &["read_file", "write_file", "bash", "search_files"],
                "standard",
                "workflow",
            ),
            specialist(
                "bi_specialist",
                "BI Specialist",
                SpecialistCategory::DataAnalytics,
                AgentLevel::Specialist,
                "Builds reporting layers, dashboards, and KPI definitions",
                "You are a BI specialist. Make metrics legible, trustworthy, and decision-ready.",
                &["read_file", "write_file", "bash", "web_search"],
                "standard",
                "pie-chart",
            ),
            specialist(
                "ml_engineer",
                "ML Engineer",
                SpecialistCategory::DataAnalytics,
                AgentLevel::Senior,
                "Implements models, pipelines, and ML-backed product features",
                "You are an ML engineer. Balance modeling ambition with pragmatic delivery and observability.",
                &["read_file", "write_file", "bash", "search_files"],
                "premium",
                "brain",
            ),
            specialist(
                "project_manager",
                "Project Manager",
                SpecialistCategory::Operations,
                AgentLevel::Manager,
                "Plans projects, tracks deliverables, and manages dependencies",
                "You are a project manager. Break work into clear steps, identify risks early, and keep teams aligned on outcomes.",
                &["read_file", "write_file", "calendar", "email"],
                "standard",
                "clipboard-list",
            ),
            specialist(
                "hr_coordinator",
                "HR Coordinator",
                SpecialistCategory::Operations,
                AgentLevel::Specialist,
                "Coordinates hiring, onboarding, and people operations",
                "You are an HR coordinator. Keep hiring and onboarding organized, humane, and process-driven.",
                &["read_file", "write_file", "email", "calendar"],
                "standard",
                "users",
            ),
            specialist(
                "customer_support_lead",
                "Customer Support Lead",
                SpecialistCategory::Operations,
                AgentLevel::Specialist,
                "Designs support workflows and resolves customer issues",
                "You are a customer support lead. Solve issues clearly, empathically, and with strong follow-through.",
                &["read_file", "write_file", "email", "memory_search"],
                "standard",
                "headphones",
            ),
            specialist(
                "sales_researcher",
                "Sales Researcher",
                SpecialistCategory::Sales,
                AgentLevel::Senior,
                "Researches competitors, markets, leads, and pricing strategies",
                "You are a sales researcher. Compile structured market intelligence with strong comparison detail and sourcing.",
                &["web_search", "web_browse", "write_file", "read_file"],
                "standard",
                "search",
            ),
            specialist(
                "proposal_writer",
                "Proposal Writer",
                SpecialistCategory::Sales,
                AgentLevel::Senior,
                "Writes business proposals, pitches, and executive sales documents",
                "You are a proposal writer. Make documents persuasive, tailored, and value-centered.",
                &["read_file", "write_file", "web_search"],
                "premium",
                "file-text",
            ),
            specialist(
                "crm_manager",
                "CRM Manager",
                SpecialistCategory::Sales,
                AgentLevel::Specialist,
                "Organizes pipeline, notes, and customer relationship workflows",
                "You are a CRM manager. Keep pipeline data clean, actionable, and easy for teams to operate from.",
                &["read_file", "write_file", "email"],
                "standard",
                "contact",
            ),
            specialist(
                "lead_qualifier",
                "Lead Qualifier",
                SpecialistCategory::Sales,
                AgentLevel::Junior,
                "Qualifies inbound leads and gathers initial context",
                "You are a lead qualifier. Be efficient, polite, and accurate when screening opportunities.",
                &["read_file", "write_file", "email", "web_search"],
                "cheap",
                "user-check",
            ),
            specialist(
                "contract_reviewer",
                "Contract Reviewer",
                SpecialistCategory::Legal,
                AgentLevel::Senior,
                "Reviews contracts, flags risks, and suggests amendments",
                "You are a contract reviewer. Analyze obligations, unusual clauses, and risks carefully. Provide analysis, not legal advice.",
                &["read_file", "write_file", "search_files"],
                "premium",
                "scale",
            ),
            specialist(
                "compliance_analyst",
                "Compliance Analyst",
                SpecialistCategory::Legal,
                AgentLevel::Specialist,
                "Maps work against policies, controls, and regulatory expectations",
                "You are a compliance analyst. Identify control gaps and explain remediation paths clearly.",
                &["read_file", "write_file", "search_files", "web_search"],
                "standard",
                "shield-check",
            ),
            specialist(
                "ip_specialist",
                "IP Specialist",
                SpecialistCategory::Legal,
                AgentLevel::Senior,
                "Assesses trademark, patent, and IP positioning issues",
                "You are an IP specialist. Evaluate intellectual property risks and summarize implications precisely.",
                &["read_file", "write_file", "web_search"],
                "premium",
                "badge-alert",
            ),
            specialist(
                "academic_researcher",
                "Academic Researcher",
                SpecialistCategory::Research,
                AgentLevel::Senior,
                "Conducts thorough research, synthesis, and source-based writing",
                "You are an academic researcher. Be methodical, source-aware, and clear about uncertainty and evidence strength.",
                &["web_search", "web_browse", "write_file", "read_file", "memory_search"],
                "premium",
                "book-open",
            ),
            specialist(
                "market_researcher",
                "Market Researcher",
                SpecialistCategory::Research,
                AgentLevel::Specialist,
                "Assesses markets, categories, demand, and positioning",
                "You are a market researcher. Build grounded market views and synthesize them into useful strategic inputs.",
                &["web_search", "web_browse", "write_file", "read_file"],
                "standard",
                "compass",
            ),
            specialist(
                "technical_writer",
                "Technical Writer",
                SpecialistCategory::Research,
                AgentLevel::Specialist,
                "Turns complex systems into clear documentation and guides",
                "You are a technical writer. Organize information so readers can understand and act without guesswork.",
                &["read_file", "write_file", "search_files"],
                "standard",
                "scroll-text",
            ),
            specialist(
                "fact_checker",
                "Fact Checker",
                SpecialistCategory::Research,
                AgentLevel::Specialist,
                "Verifies claims, cross-checks sources, and flags uncertainty",
                "You are a fact checker. Cross-reference carefully, distinguish fact from inference, and cite sources or confidence limits.",
                &["web_search", "web_browse", "read_file"],
                "cheap",
                "check-circle",
            ),
        ];

        for profile in defs {
            self.add(profile);
        }
    }
}

impl Default for SpecialistRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn specialist(
    id: &str,
    name: &str,
    category: SpecialistCategory,
    level: AgentLevel,
    description: &str,
    system_prompt: &str,
    default_tools: &[&str],
    default_model_tier: &str,
    icon: &str,
) -> SpecialistProfile {
    SpecialistProfile {
        id: id.to_string(),
        name: name.to_string(),
        category,
        level,
        description: description.to_string(),
        system_prompt: system_prompt.to_string(),
        default_tools: default_tools.iter().map(|tool| tool.to_string()).collect(),
        default_model_tier: default_model_tier.to_string(),
        icon: icon.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_twenty_plus_specialists() {
        let registry = SpecialistRegistry::new();
        assert!(registry.list().len() >= 20);
    }

    #[test]
    fn registry_has_expected_categories() {
        let registry = SpecialistRegistry::new();
        let mut categories = registry
            .list()
            .into_iter()
            .map(|profile| profile.category)
            .collect::<Vec<_>>();
        categories.sort_by(|left, right| left.label().cmp(right.label()));
        categories.dedup();
        assert!(categories.len() >= 8);
    }

    #[test]
    fn lookup_by_id_works() {
        let registry = SpecialistRegistry::new();
        let specialist = registry.get("backend_dev").expect("missing backend_dev");
        assert_eq!(specialist.name, "Backend Developer");
    }
}
