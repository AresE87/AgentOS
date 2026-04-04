import type { AgentAssignment, TaskDAG } from './model';
import { createDraftNode } from './model';

export interface MissionTemplateDefinition {
  id: string;
  title: string;
  description: string;
  promptLabel: string;
  promptPlaceholder: string;
  agentCount: number;
}

function assignment(level: AgentAssignment['level'], specialist: string, specialistName: string): AgentAssignment {
  return {
    level,
    specialist,
    specialist_name: specialistName,
    model_override: null,
    mesh_node: null,
  };
}

function emptyDag(): TaskDAG {
  return { nodes: {}, edges: [] };
}

export const missionTemplates: MissionTemplateDefinition[] = [
  {
    id: 'market_research',
    title: 'Investigación de Mercado',
    description: 'Investigá competidores, estructurá hallazgos y escribí el brief ejecutivo.',
    promptLabel: '¿Qué mercado o competidores?',
    promptPlaceholder: 'GitHub Copilot, Cursor, Windsurf...',
    agentCount: 3,
  },
  {
    id: 'code_review',
    title: 'Code Review + Tests',
    description: 'Revisá calidad de código, verificá seguridad y proponé mejoras de cobertura.',
    promptLabel: '¿Qué archivo o directorio?',
    promptPlaceholder: 'frontend/src/pages/dashboard',
    agentCount: 4,
  },
  {
    id: 'content_pipeline',
    title: 'Pipeline de Contenido',
    description: 'Investigá el tema, optimizá para SEO, redactá y publica contenido.',
    promptLabel: '¿Qué tema querés cubrir?',
    promptPlaceholder: 'AI workflow automation for operations teams',
    agentCount: 4,
  },
  {
    id: 'due_diligence',
    title: 'Due Diligence',
    description: 'Recopilá contexto financiero, legal y contractual en un reporte de diligencia.',
    promptLabel: '¿Qué empresa?',
    promptPlaceholder: 'Figma',
    agentCount: 4,
  },
  {
    id: 'email_campaign',
    title: 'Campaña de Email',
    description: 'Investigá la audiencia, escribí variantes y prepará la secuencia lista para enviar.',
    promptLabel: '¿Qué producto y audiencia?',
    promptPlaceholder: 'AgentOS for SaaS founders',
    agentCount: 5,
  },
  {
    id: 'design_sprint',
    title: 'Design Sprint',
    description: 'Investigá fricciones UX, diseñá un concepto, armá la UI y verificá calidad.',
    promptLabel: '¿Qué problema de UX?',
    promptPlaceholder: 'Confusing onboarding for first-time users',
    agentCount: 4,
  },
];

export function buildTemplateDag(templateId: string, context: string): TaskDAG {
  const dag = emptyDag();

  if (templateId === 'market_research') {
    dag.nodes.research = createDraftNode({
      id: 'research',
      title: 'Research competitors',
      description: `Research ${context} and capture pricing, positioning, and differentiators.`,
      assignment: assignment('Senior', 'sales_researcher', 'Sales Researcher'),
      allowed_tools: ['web_search', 'web_browse', 'read_file', 'write_file'],
      position: { x: 100, y: 140 },
    });
    dag.nodes.analysis = createDraftNode({
      id: 'analysis',
      title: 'Analyze patterns',
      description: `Turn the research on ${context} into a structured comparison table.`,
      assignment: assignment('Specialist', 'data_analyst', 'Data Analyst'),
      allowed_tools: ['read_file', 'write_file', 'bash', 'search_files'],
      position: { x: 430, y: 140 },
    });
    dag.nodes.report = createDraftNode({
      id: 'report',
      title: 'Write executive summary',
      description: `Write a concise executive report for ${context} using the structured findings.`,
      assignment: assignment('Senior', 'proposal_writer', 'Proposal Writer'),
      allowed_tools: ['read_file', 'write_file', 'web_search'],
      position: { x: 760, y: 140 },
    });
    dag.edges.push(
      { from: 'research', to: 'analysis', edge_type: 'DataFlow' },
      { from: 'analysis', to: 'report', edge_type: 'DataFlow' },
    );
    return dag;
  }

  if (templateId === 'code_review') {
    dag.nodes.reader = createDraftNode({
      id: 'reader',
      title: 'Read target scope',
      description: `Inspect ${context} and summarize the architecture, hotspots, and review scope.`,
      assignment: assignment('Senior', 'backend_dev', 'Backend Developer'),
      allowed_tools: ['read_file', 'search_files', 'bash'],
      position: { x: 80, y: 120 },
    });
    dag.nodes.tests = createDraftNode({
      id: 'tests',
      title: 'Review test coverage',
      description: `Assess existing tests around ${context} and propose missing cases.`,
      assignment: assignment('Specialist', 'qa_tester', 'QA Tester'),
      allowed_tools: ['read_file', 'write_file', 'bash', 'search_files'],
      position: { x: 420, y: 60 },
    });
    dag.nodes.security = createDraftNode({
      id: 'security',
      title: 'Review security risk',
      description: `Check ${context} for security risks, unsafe assumptions, and missing safeguards.`,
      assignment: assignment('Senior', 'software_architect', 'Software Architect'),
      allowed_tools: ['read_file', 'search_files', 'write_file'],
      position: { x: 420, y: 240 },
    });
    dag.nodes.summary = createDraftNode({
      id: 'summary',
      title: 'Write review summary',
      description: `Combine findings for ${context} into one prioritized review summary.`,
      assignment: assignment('Senior', 'technical_writer', 'Technical Writer'),
      allowed_tools: ['read_file', 'write_file'],
      position: { x: 770, y: 140 },
    });
    dag.edges.push(
      { from: 'reader', to: 'tests', edge_type: 'Dependency' },
      { from: 'reader', to: 'security', edge_type: 'Dependency' },
      { from: 'tests', to: 'summary', edge_type: 'DataFlow' },
      { from: 'security', to: 'summary', edge_type: 'DataFlow' },
    );
    return dag;
  }

  if (templateId === 'content_pipeline') {
    dag.nodes.research = createDraftNode({
      id: 'research',
      title: 'Topic research',
      description: `Research audience intent and supporting facts for ${context}.`,
      assignment: assignment('Senior', 'market_researcher', 'Market Researcher'),
      allowed_tools: ['web_search', 'web_browse', 'read_file', 'write_file'],
      position: { x: 90, y: 140 },
    });
    dag.nodes.seo = createDraftNode({
      id: 'seo',
      title: 'SEO brief',
      description: `Define keyword clusters, search intent, and content angles for ${context}.`,
      assignment: assignment('Specialist', 'seo_specialist', 'SEO Specialist'),
      allowed_tools: ['web_search', 'web_browse', 'write_file', 'read_file'],
      position: { x: 390, y: 140 },
    });
    dag.nodes.write = createDraftNode({
      id: 'write',
      title: 'Draft article',
      description: `Write the main content piece for ${context} using the research and SEO brief.`,
      assignment: assignment('Specialist', 'content_marketer', 'Content Marketer'),
      allowed_tools: ['read_file', 'write_file', 'web_search'],
      position: { x: 690, y: 140 },
    });
    dag.nodes.edit = createDraftNode({
      id: 'edit',
      title: 'Edit and polish',
      description: `Edit the draft for clarity, structure, and persuasion.`,
      assignment: assignment('Specialist', 'copywriter', 'Copywriter'),
      allowed_tools: ['read_file', 'write_file'],
      position: { x: 990, y: 140 },
    });
    dag.edges.push(
      { from: 'research', to: 'seo', edge_type: 'DataFlow' },
      { from: 'seo', to: 'write', edge_type: 'DataFlow' },
      { from: 'write', to: 'edit', edge_type: 'DataFlow' },
    );
    return dag;
  }

  if (templateId === 'due_diligence') {
    dag.nodes.company = createDraftNode({
      id: 'company',
      title: 'Company research',
      description: `Research company positioning, leadership, and product footprint for ${context}.`,
      assignment: assignment('Senior', 'market_researcher', 'Market Researcher'),
      allowed_tools: ['web_search', 'web_browse', 'read_file', 'write_file'],
      position: { x: 90, y: 90 },
    });
    dag.nodes.finance = createDraftNode({
      id: 'finance',
      title: 'Financial review',
      description: `Collect financial indicators and risks related to ${context}.`,
      assignment: assignment('Senior', 'financial_analyst', 'Financial Analyst'),
      allowed_tools: ['web_search', 'web_browse', 'read_file', 'write_file', 'bash'],
      position: { x: 90, y: 260 },
    });
    dag.nodes.legal = createDraftNode({
      id: 'legal',
      title: 'Contract and legal risks',
      description: `Review public legal, compliance, and contract-related risks for ${context}.`,
      assignment: assignment('Senior', 'contract_reviewer', 'Contract Reviewer'),
      allowed_tools: ['read_file', 'write_file', 'search_files', 'web_search'],
      position: { x: 450, y: 175 },
    });
    dag.nodes.report = createDraftNode({
      id: 'report',
      title: 'Diligence report',
      description: `Compile a due diligence summary for ${context}.`,
      assignment: assignment('Senior', 'proposal_writer', 'Proposal Writer'),
      allowed_tools: ['read_file', 'write_file'],
      position: { x: 820, y: 175 },
    });
    dag.edges.push(
      { from: 'company', to: 'legal', edge_type: 'DataFlow' },
      { from: 'finance', to: 'legal', edge_type: 'DataFlow' },
      { from: 'legal', to: 'report', edge_type: 'DataFlow' },
    );
    return dag;
  }

  if (templateId === 'email_campaign') {
    dag.nodes.research = createDraftNode({
      id: 'research',
      title: 'Audience research',
      description: `Research the audience and positioning for ${context}.`,
      assignment: assignment('Senior', 'sales_researcher', 'Sales Researcher'),
      allowed_tools: ['web_search', 'web_browse', 'read_file', 'write_file'],
      position: { x: 90, y: 170 },
    });
    ['variant_a', 'variant_b', 'variant_c'].forEach((id, index) => {
      dag.nodes[id] = createDraftNode({
        id,
        title: `Write ${id.replace('variant_', 'variant ').toUpperCase()}`,
        description: `Write a distinct email variant for ${context}.`,
        assignment: assignment('Specialist', 'copywriter', 'Copywriter'),
        allowed_tools: ['read_file', 'write_file'],
        position: { x: 430 + index * 260, y: 70 + index * 80 },
      });
      dag.edges.push({ from: 'research', to: id, edge_type: 'DataFlow' });
    });
    dag.nodes.send = createDraftNode({
      id: 'send',
      title: 'Prepare send-ready sequence',
      description: `Choose the best variants and prepare the campaign sequence for ${context}.`,
      assignment: assignment('Junior', 'email_writer', 'Email Writer'),
      allowed_tools: ['email', 'read_file', 'write_file'],
      position: { x: 1250, y: 170 },
    });
    dag.edges.push(
      { from: 'variant_a', to: 'send', edge_type: 'DataFlow' },
      { from: 'variant_b', to: 'send', edge_type: 'DataFlow' },
      { from: 'variant_c', to: 'send', edge_type: 'DataFlow' },
    );
    return dag;
  }

  dag.nodes.research = createDraftNode({
    id: 'research',
    title: 'UX research',
    description: `Investigate the UX challenge for ${context}.`,
    assignment: assignment('Senior', 'ux_researcher', 'UX Researcher'),
    allowed_tools: ['web_search', 'web_browse', 'read_file', 'write_file'],
    position: { x: 90, y: 170 },
  });
  dag.nodes.design = createDraftNode({
    id: 'design',
    title: 'Design concept',
    description: `Design a UI concept that addresses ${context}.`,
    assignment: assignment('Senior', 'ui_designer', 'UI Designer'),
    allowed_tools: ['read_file', 'write_file'],
    position: { x: 430, y: 170 },
  });
  dag.nodes.build = createDraftNode({
    id: 'build',
    title: 'Implement frontend',
    description: `Implement the proposed UI solution for ${context}.`,
    assignment: assignment('Senior', 'frontend_dev', 'Frontend Developer'),
    allowed_tools: ['read_file', 'write_file', 'edit_file', 'search_files', 'bash'],
    position: { x: 770, y: 170 },
  });
  dag.nodes.qa = createDraftNode({
    id: 'qa',
    title: 'QA pass',
    description: `Validate the shipped UX improvements for ${context}.`,
    assignment: assignment('Specialist', 'qa_tester', 'QA Tester'),
    allowed_tools: ['read_file', 'write_file', 'bash', 'search_files'],
    position: { x: 1110, y: 170 },
  });
  dag.edges.push(
    { from: 'research', to: 'design', edge_type: 'DataFlow' },
    { from: 'design', to: 'build', edge_type: 'DataFlow' },
    { from: 'build', to: 'qa', edge_type: 'Dependency' },
  );
  return dag;
}
