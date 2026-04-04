use crate::coordinator::types::{
    AgentAssignment, AgentLevel, DAGEdge, DAGNode, EdgeType, NodePosition, SubtaskStatus, TaskDAG,
};

pub struct MissionTemplates;

impl MissionTemplates {
    pub fn build(template_id: &str, context: &str) -> Option<TaskDAG> {
        let context = context.trim();
        let dag = match template_id {
            "market_research" => market_research(context),
            "code_review" => code_review(context),
            "content_pipeline" => content_pipeline(context),
            "due_diligence" => due_diligence(context),
            "email_campaign" => email_campaign(context),
            "design_sprint" => design_sprint(context),
            _ => return None,
        };
        Some(dag)
    }
}

fn market_research(context: &str) -> TaskDAG {
    let mut dag = TaskDAG::new();
    dag.add_node(node(
        "research",
        "Research competitors",
        &format!(
            "Research {} and capture pricing, positioning, and differentiators.",
            context
        ),
        assignment(AgentLevel::Senior, "sales_researcher", "Sales Researcher"),
        &["web_search", "web_browse", "read_file", "write_file"],
        100.0,
        140.0,
    ));
    dag.add_node(node(
        "analysis",
        "Analyze patterns",
        &format!(
            "Turn the research on {} into a structured comparison table.",
            context
        ),
        assignment(AgentLevel::Specialist, "data_analyst", "Data Analyst"),
        &["read_file", "write_file", "bash", "search_files"],
        430.0,
        140.0,
    ));
    dag.add_node(node(
        "report",
        "Write executive summary",
        &format!(
            "Write a concise executive report for {} using the structured findings.",
            context
        ),
        assignment(AgentLevel::Senior, "proposal_writer", "Proposal Writer"),
        &["read_file", "write_file", "web_search"],
        760.0,
        140.0,
    ));
    dag.add_edge(DAGEdge {
        from: "research".into(),
        to: "analysis".into(),
        edge_type: EdgeType::DataFlow,
    });
    dag.add_edge(DAGEdge {
        from: "analysis".into(),
        to: "report".into(),
        edge_type: EdgeType::DataFlow,
    });
    dag
}

fn code_review(context: &str) -> TaskDAG {
    let mut dag = TaskDAG::new();
    dag.add_node(node(
        "reader",
        "Read target scope",
        &format!(
            "Inspect {} and summarize the architecture, hotspots, and review scope.",
            context
        ),
        assignment(AgentLevel::Senior, "backend_dev", "Backend Developer"),
        &["read_file", "search_files", "bash"],
        80.0,
        120.0,
    ));
    dag.add_node(node(
        "tests",
        "Review test coverage",
        &format!(
            "Assess existing tests around {} and propose missing cases.",
            context
        ),
        assignment(AgentLevel::Specialist, "qa_tester", "QA Tester"),
        &["read_file", "write_file", "bash", "search_files"],
        420.0,
        60.0,
    ));
    dag.add_node(node(
        "security",
        "Review security risk",
        &format!(
            "Check {} for security risks, unsafe assumptions, and missing safeguards.",
            context
        ),
        assignment(
            AgentLevel::Senior,
            "software_architect",
            "Software Architect",
        ),
        &["read_file", "search_files", "write_file"],
        420.0,
        240.0,
    ));
    dag.add_node(node(
        "summary",
        "Write review summary",
        &format!(
            "Combine findings for {} into one prioritized review summary.",
            context
        ),
        assignment(AgentLevel::Senior, "technical_writer", "Technical Writer"),
        &["read_file", "write_file"],
        770.0,
        140.0,
    ));
    dag.add_edge(edge("reader", "tests", EdgeType::Dependency));
    dag.add_edge(edge("reader", "security", EdgeType::Dependency));
    dag.add_edge(edge("tests", "summary", EdgeType::DataFlow));
    dag.add_edge(edge("security", "summary", EdgeType::DataFlow));
    dag
}

fn content_pipeline(context: &str) -> TaskDAG {
    let mut dag = TaskDAG::new();
    dag.add_node(node(
        "research",
        "Topic research",
        &format!(
            "Research audience intent and supporting facts for {}.",
            context
        ),
        assignment(AgentLevel::Senior, "market_researcher", "Market Researcher"),
        &["web_search", "web_browse", "read_file", "write_file"],
        90.0,
        140.0,
    ));
    dag.add_node(node(
        "seo",
        "SEO brief",
        &format!(
            "Define keyword clusters, search intent, and content angles for {}.",
            context
        ),
        assignment(AgentLevel::Specialist, "seo_specialist", "SEO Specialist"),
        &["web_search", "web_browse", "write_file", "read_file"],
        390.0,
        140.0,
    ));
    dag.add_node(node(
        "write",
        "Draft article",
        &format!(
            "Write the main content piece for {} using the research and SEO brief.",
            context
        ),
        assignment(
            AgentLevel::Specialist,
            "content_marketer",
            "Content Marketer",
        ),
        &["read_file", "write_file", "web_search"],
        690.0,
        140.0,
    ));
    dag.add_node(node(
        "edit",
        "Edit and polish",
        "Edit the draft for clarity, structure, and persuasion.",
        assignment(AgentLevel::Specialist, "copywriter", "Copywriter"),
        &["read_file", "write_file"],
        990.0,
        140.0,
    ));
    dag.add_edge(edge("research", "seo", EdgeType::DataFlow));
    dag.add_edge(edge("seo", "write", EdgeType::DataFlow));
    dag.add_edge(edge("write", "edit", EdgeType::DataFlow));
    dag
}

fn due_diligence(context: &str) -> TaskDAG {
    let mut dag = TaskDAG::new();
    dag.add_node(node(
        "company",
        "Company research",
        &format!(
            "Research company positioning, leadership, and product footprint for {}.",
            context
        ),
        assignment(AgentLevel::Senior, "market_researcher", "Market Researcher"),
        &["web_search", "web_browse", "read_file", "write_file"],
        90.0,
        90.0,
    ));
    dag.add_node(node(
        "finance",
        "Financial review",
        &format!(
            "Collect financial indicators and risks related to {}.",
            context
        ),
        assignment(AgentLevel::Senior, "financial_analyst", "Financial Analyst"),
        &[
            "web_search",
            "web_browse",
            "read_file",
            "write_file",
            "bash",
        ],
        90.0,
        260.0,
    ));
    dag.add_node(node(
        "legal",
        "Contract and legal risks",
        &format!(
            "Review public legal, compliance, and contract-related risks for {}.",
            context
        ),
        assignment(AgentLevel::Senior, "contract_reviewer", "Contract Reviewer"),
        &["read_file", "write_file", "search_files", "web_search"],
        450.0,
        175.0,
    ));
    dag.add_node(node(
        "report",
        "Diligence report",
        &format!("Compile a due diligence summary for {}.", context),
        assignment(AgentLevel::Senior, "proposal_writer", "Proposal Writer"),
        &["read_file", "write_file"],
        820.0,
        175.0,
    ));
    dag.add_edge(edge("company", "legal", EdgeType::DataFlow));
    dag.add_edge(edge("finance", "legal", EdgeType::DataFlow));
    dag.add_edge(edge("legal", "report", EdgeType::DataFlow));
    dag
}

fn email_campaign(context: &str) -> TaskDAG {
    let mut dag = TaskDAG::new();
    dag.add_node(node(
        "research",
        "Audience research",
        &format!("Research the audience and positioning for {}.", context),
        assignment(AgentLevel::Senior, "sales_researcher", "Sales Researcher"),
        &["web_search", "web_browse", "read_file", "write_file"],
        90.0,
        170.0,
    ));
    for (index, variant) in ["variant_a", "variant_b", "variant_c"].iter().enumerate() {
        dag.add_node(node(
            variant,
            &format!("Write {}", variant.replace('_', " ").to_uppercase()),
            &format!("Write a distinct email variant for {}.", context),
            assignment(AgentLevel::Specialist, "copywriter", "Copywriter"),
            &["read_file", "write_file"],
            430.0 + index as f32 * 260.0,
            70.0 + index as f32 * 80.0,
        ));
        dag.add_edge(edge("research", variant, EdgeType::DataFlow));
    }
    dag.add_node(node(
        "send",
        "Prepare send-ready sequence",
        &format!(
            "Choose the best variants and prepare the campaign sequence for {}.",
            context
        ),
        assignment(AgentLevel::Junior, "email_writer", "Email Writer"),
        &["email", "read_file", "write_file"],
        1250.0,
        170.0,
    ));
    dag.add_edge(edge("variant_a", "send", EdgeType::DataFlow));
    dag.add_edge(edge("variant_b", "send", EdgeType::DataFlow));
    dag.add_edge(edge("variant_c", "send", EdgeType::DataFlow));
    dag
}

fn design_sprint(context: &str) -> TaskDAG {
    let mut dag = TaskDAG::new();
    dag.add_node(node(
        "research",
        "UX research",
        &format!("Investigate the UX challenge for {}.", context),
        assignment(AgentLevel::Senior, "ux_researcher", "UX Researcher"),
        &["web_search", "web_browse", "read_file", "write_file"],
        90.0,
        170.0,
    ));
    dag.add_node(node(
        "design",
        "Design concept",
        &format!("Design a UI concept that addresses {}.", context),
        assignment(AgentLevel::Senior, "ui_designer", "UI Designer"),
        &["read_file", "write_file"],
        430.0,
        170.0,
    ));
    dag.add_node(node(
        "build",
        "Implement frontend",
        &format!("Implement the proposed UI solution for {}.", context),
        assignment(AgentLevel::Senior, "frontend_dev", "Frontend Developer"),
        &[
            "read_file",
            "write_file",
            "edit_file",
            "search_files",
            "bash",
        ],
        770.0,
        170.0,
    ));
    dag.add_node(node(
        "qa",
        "QA pass",
        &format!("Validate the shipped UX improvements for {}.", context),
        assignment(AgentLevel::Specialist, "qa_tester", "QA Tester"),
        &["read_file", "write_file", "bash", "search_files"],
        1110.0,
        170.0,
    ));
    dag.add_edge(edge("research", "design", EdgeType::DataFlow));
    dag.add_edge(edge("design", "build", EdgeType::DataFlow));
    dag.add_edge(edge("build", "qa", EdgeType::Dependency));
    dag
}

fn assignment(level: AgentLevel, specialist: &str, specialist_name: &str) -> AgentAssignment {
    AgentAssignment {
        level,
        specialist: Some(specialist.to_string()),
        specialist_name: Some(specialist_name.to_string()),
        model_override: None,
        mesh_node: None,
    }
}

fn node(
    id: &str,
    title: &str,
    description: &str,
    assignment: AgentAssignment,
    allowed_tools: &[&str],
    x: f32,
    y: f32,
) -> DAGNode {
    DAGNode {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        assignment,
        allowed_tools: allowed_tools.iter().map(|tool| tool.to_string()).collect(),
        status: SubtaskStatus::Queued,
        progress: 0.0,
        last_message: None,
        result: None,
        error: None,
        cost: 0.0,
        tokens_in: 0,
        tokens_out: 0,
        elapsed_ms: 0,
        started_at: None,
        completed_at: None,
        retry_count: 0,
        max_retries: 2,
        position: Some(NodePosition { x, y }),
        awaiting_approval: false,
        approved_to_run: false,
    }
}

fn edge(from: &str, to: &str, edge_type: EdgeType) -> DAGEdge {
    DAGEdge {
        from: from.to_string(),
        to: to.to_string(),
        edge_type,
    }
}

#[cfg(test)]
mod tests {
    use super::MissionTemplates;

    #[test]
    fn builds_known_template() {
        let dag = MissionTemplates::build("market_research", "AgentOS").expect("template");
        assert_eq!(dag.nodes.len(), 3);
        assert_eq!(dag.edges.len(), 2);
    }

    #[test]
    fn unknown_template_returns_none() {
        assert!(MissionTemplates::build("missing", "ctx").is_none());
    }
}
