use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub agents: Vec<AgentConfig>,
    pub connectors_required: Vec<String>,
    pub category: String,
    pub setup_steps: Vec<SetupStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub role: String,
    pub specialist: String,
    pub level: String,
    pub tools: Vec<String>,
    pub schedule: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStep {
    pub step: u32,
    pub title: String,
    pub description: String,
    pub field_type: String,
    pub field_key: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub template_id: String,
    pub name: String,
    pub settings: serde_json::Value,
    pub active: bool,
    pub created_at: String,
}

pub fn all_templates() -> Vec<TeamTemplate> {
    vec![
        marketing_team(),
        sales_team(),
        support_team(),
        content_team(),
        finance_team(),
    ]
}

pub fn get_template(id: &str) -> Option<TeamTemplate> {
    all_templates().into_iter().find(|t| t.id == id)
}

// ---------------------------------------------------------------------------
// Marketing Team
// ---------------------------------------------------------------------------
fn marketing_team() -> TeamTemplate {
    TeamTemplate {
        id: "marketing".into(),
        name: "Equipo de Marketing".into(),
        description: "Equipo completo de marketing digital: gestiona redes sociales, crea contenido, \
                       analiza metricas y optimiza tu presencia online automaticamente.".into(),
        icon: "megaphone".into(),
        category: "marketing".into(),
        connectors_required: vec!["twitter".into(), "linkedin".into(), "reddit".into()],
        agents: vec![
            AgentConfig {
                role: "director".into(),
                specialist: "Social Media Director".into(),
                level: "senior".into(),
                tools: vec!["social_post".into(), "social_schedule".into(), "analytics_read".into()],
                schedule: Some("0 9 * * 1-5".into()),
                description: "Dirige la estrategia de redes sociales, programa publicaciones \
                              y coordina al equipo de contenido.".into(),
            },
            AgentConfig {
                role: "writer".into(),
                specialist: "Content Writer".into(),
                level: "mid".into(),
                tools: vec!["text_generate".into(), "image_generate".into(), "web_browse".into()],
                schedule: Some("0 10 * * 1-5".into()),
                description: "Redacta publicaciones, articulos de blog y copy publicitario \
                              adaptado a cada plataforma.".into(),
            },
            AgentConfig {
                role: "community".into(),
                specialist: "Community Manager".into(),
                level: "mid".into(),
                tools: vec!["social_reply".into(), "social_mentions".into(), "sentiment_analyze".into()],
                schedule: Some("0 */2 * * *".into()),
                description: "Responde comentarios, gestiona la comunidad y monitorea \
                              menciones de la marca en tiempo real.".into(),
            },
            AgentConfig {
                role: "seo".into(),
                specialist: "SEO Specialist".into(),
                level: "mid".into(),
                tools: vec!["web_browse".into(), "text_generate".into(), "analytics_read".into()],
                schedule: Some("0 8 * * 1".into()),
                description: "Optimiza contenido para motores de busqueda, investiga palabras \
                              clave y mejora el posicionamiento organico.".into(),
            },
            AgentConfig {
                role: "analytics".into(),
                specialist: "Analytics Analyst".into(),
                level: "senior".into(),
                tools: vec!["analytics_read".into(), "report_generate".into(), "data_export".into()],
                schedule: Some("0 17 * * 5".into()),
                description: "Genera reportes semanales de rendimiento, identifica tendencias \
                              y recomienda ajustes a la estrategia.".into(),
            },
        ],
        setup_steps: vec![
            SetupStep {
                step: 1,
                title: "Conectar Redes Sociales".into(),
                description: "Autoriza el acceso a tus cuentas de Twitter, LinkedIn y Reddit \
                              para que el equipo pueda publicar y monitorear.".into(),
                field_type: "oauth".into(),
                field_key: "social_accounts".into(),
                required: true,
            },
            SetupStep {
                step: 2,
                title: "Definir Voz de Marca".into(),
                description: "Describe el tono y estilo de comunicacion de tu marca \
                              (profesional, casual, tecnico, etc.).".into(),
                field_type: "text".into(),
                field_key: "brand_voice".into(),
                required: true,
            },
            SetupStep {
                step: 3,
                title: "Frecuencia de Publicacion".into(),
                description: "Selecciona cuantas publicaciones por semana quieres en cada plataforma.".into(),
                field_type: "select".into(),
                field_key: "posting_frequency".into(),
                required: true,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// Sales Team
// ---------------------------------------------------------------------------
fn sales_team() -> TeamTemplate {
    TeamTemplate {
        id: "sales".into(),
        name: "Equipo de Ventas".into(),
        description: "Automatiza tu pipeline de ventas: investigacion de leads, propuestas, \
                       outreach por email, gestion de CRM y seguimiento automatico.".into(),
        icon: "dollar-sign".into(),
        category: "ventas".into(),
        connectors_required: vec!["linkedin".into(), "gmail".into(), "calendar".into()],
        agents: vec![
            AgentConfig {
                role: "researcher".into(),
                specialist: "Lead Researcher".into(),
                level: "mid".into(),
                tools: vec!["web_browse".into(), "linkedin_search".into(), "data_enrich".into()],
                schedule: Some("0 8 * * 1-5".into()),
                description: "Investiga y califica leads potenciales usando LinkedIn, \
                              sitios web corporativos y bases de datos publicas.".into(),
            },
            AgentConfig {
                role: "proposal".into(),
                specialist: "Proposal Writer".into(),
                level: "senior".into(),
                tools: vec!["text_generate".into(), "template_render".into(), "file_create".into()],
                schedule: None,
                description: "Redacta propuestas comerciales personalizadas basadas en \
                              la investigacion del lead y los servicios disponibles.".into(),
            },
            AgentConfig {
                role: "outreach".into(),
                specialist: "Email Outreach".into(),
                level: "mid".into(),
                tools: vec!["email_send".into(), "email_draft".into(), "template_render".into()],
                schedule: Some("0 9 * * 1-5".into()),
                description: "Envia secuencias de email personalizadas a leads calificados \
                              siguiendo las plantillas de outreach configuradas.".into(),
            },
            AgentConfig {
                role: "crm".into(),
                specialist: "CRM Manager".into(),
                level: "mid".into(),
                tools: vec!["data_store".into(), "analytics_read".into(), "report_generate".into()],
                schedule: Some("0 18 * * 1-5".into()),
                description: "Mantiene actualizado el CRM con estados de leads, \
                              interacciones y probabilidades de cierre.".into(),
            },
            AgentConfig {
                role: "followup".into(),
                specialist: "Follow-up Agent".into(),
                level: "junior".into(),
                tools: vec!["email_send".into(), "calendar_create".into(), "reminder_set".into()],
                schedule: Some("0 10 * * 1,3,5".into()),
                description: "Realiza seguimiento automatico a leads que no han respondido, \
                              programa reuniones y envia recordatorios.".into(),
            },
        ],
        setup_steps: vec![
            SetupStep {
                step: 1,
                title: "Conectar Email".into(),
                description: "Conecta tu cuenta de Gmail o correo corporativo para enviar \
                              y recibir emails de outreach.".into(),
                field_type: "oauth".into(),
                field_key: "email_account".into(),
                required: true,
            },
            SetupStep {
                step: 2,
                title: "Industrias Objetivo".into(),
                description: "Define las industrias y tipos de empresa que quieres prospectar \
                              (tecnologia, finanzas, salud, etc.).".into(),
                field_type: "text".into(),
                field_key: "target_industries".into(),
                required: true,
            },
            SetupStep {
                step: 3,
                title: "Plantillas de Outreach".into(),
                description: "Personaliza las plantillas de email para el primer contacto, \
                              seguimiento y cierre.".into(),
                field_type: "text".into(),
                field_key: "outreach_templates".into(),
                required: false,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// Support Team
// ---------------------------------------------------------------------------
fn support_team() -> TeamTemplate {
    TeamTemplate {
        id: "support".into(),
        name: "Equipo de Soporte".into(),
        description: "Soporte al cliente automatizado: triaje de tickets, respuestas L1, \
                       documentacion, escalaciones inteligentes y monitoreo de satisfaccion.".into(),
        icon: "headphones".into(),
        category: "soporte".into(),
        connectors_required: vec!["email".into(), "discord".into()],
        agents: vec![
            AgentConfig {
                role: "triage".into(),
                specialist: "Ticket Triage".into(),
                level: "mid".into(),
                tools: vec!["email_read".into(), "classify_intent".into(), "priority_assign".into()],
                schedule: Some("*/15 * * * *".into()),
                description: "Clasifica tickets entrantes por prioridad, categoria y urgencia, \
                              asignandolos al agente correcto automaticamente.".into(),
            },
            AgentConfig {
                role: "l1_support".into(),
                specialist: "L1 Support".into(),
                level: "junior".into(),
                tools: vec!["knowledge_search".into(), "email_reply".into(), "template_render".into()],
                schedule: Some("*/30 * * * *".into()),
                description: "Responde preguntas frecuentes y problemas comunes usando la \
                              base de conocimiento, escalando si no puede resolver.".into(),
            },
            AgentConfig {
                role: "doc_writer".into(),
                specialist: "Doc Writer".into(),
                level: "mid".into(),
                tools: vec!["text_generate".into(), "knowledge_store".into(), "file_create".into()],
                schedule: Some("0 16 * * 5".into()),
                description: "Crea y actualiza articulos de ayuda, FAQs y documentacion \
                              tecnica basandose en los tickets resueltos.".into(),
            },
            AgentConfig {
                role: "escalation".into(),
                specialist: "Escalation Manager".into(),
                level: "senior".into(),
                tools: vec!["escalation_create".into(), "notify_human".into(), "priority_assign".into()],
                schedule: Some("0 */4 * * *".into()),
                description: "Gestiona escalaciones complejas, notifica al equipo humano \
                              y asegura que los SLAs se cumplan.".into(),
            },
            AgentConfig {
                role: "satisfaction".into(),
                specialist: "Satisfaction Monitor".into(),
                level: "mid".into(),
                tools: vec!["survey_send".into(), "sentiment_analyze".into(), "report_generate".into()],
                schedule: Some("0 9 * * 1".into()),
                description: "Envia encuestas de satisfaccion, analiza el sentimiento \
                              de las respuestas y genera reportes CSAT semanales.".into(),
            },
        ],
        setup_steps: vec![
            SetupStep {
                step: 1,
                title: "Conectar Email de Soporte".into(),
                description: "Conecta la direccion de email donde recibes tickets de soporte \
                              (soporte@tuempresa.com).".into(),
                field_type: "oauth".into(),
                field_key: "support_email".into(),
                required: true,
            },
            SetupStep {
                step: 2,
                title: "Base de Conocimiento".into(),
                description: "Sube documentos, FAQs o URLs de tu centro de ayuda existente \
                              para que los agentes puedan consultar.".into(),
                field_type: "text".into(),
                field_key: "knowledge_base".into(),
                required: true,
            },
            SetupStep {
                step: 3,
                title: "Reglas de Escalacion".into(),
                description: "Define cuando un ticket debe escalarse a un humano \
                              (prioridad alta, temas sensibles, etc.).".into(),
                field_type: "text".into(),
                field_key: "escalation_rules".into(),
                required: false,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// Content Team
// ---------------------------------------------------------------------------
fn content_team() -> TeamTemplate {
    TeamTemplate {
        id: "content".into(),
        name: "Equipo de Contenido".into(),
        description: "Produccion de contenido automatizada: investigacion, redaccion, edicion, \
                       optimizacion SEO y distribucion en multiples canales.".into(),
        icon: "pen-tool".into(),
        category: "contenido".into(),
        connectors_required: vec!["web_browse".into(), "email".into()],
        agents: vec![
            AgentConfig {
                role: "researcher".into(),
                specialist: "Research Agent".into(),
                level: "mid".into(),
                tools: vec!["web_browse".into(), "web_search".into(), "data_extract".into()],
                schedule: Some("0 7 * * 1-5".into()),
                description: "Investiga temas trending, analiza competidores y recopila \
                              datos relevantes para la creacion de contenido.".into(),
            },
            AgentConfig {
                role: "writer".into(),
                specialist: "Writer".into(),
                level: "senior".into(),
                tools: vec!["text_generate".into(), "image_generate".into(), "file_create".into()],
                schedule: Some("0 9 * * 1-5".into()),
                description: "Redacta articulos, blogs, newsletters y contenido largo \
                              siguiendo las guias de estilo y tono de marca.".into(),
            },
            AgentConfig {
                role: "editor".into(),
                specialist: "Editor".into(),
                level: "senior".into(),
                tools: vec!["text_generate".into(), "grammar_check".into(), "style_review".into()],
                schedule: Some("0 14 * * 1-5".into()),
                description: "Revisa y mejora el contenido generado, corrige errores \
                              gramaticales y asegura consistencia de estilo.".into(),
            },
            AgentConfig {
                role: "seo_optimizer".into(),
                specialist: "SEO Optimizer".into(),
                level: "mid".into(),
                tools: vec!["seo_analyze".into(), "keyword_research".into(), "text_generate".into()],
                schedule: Some("0 15 * * 1-5".into()),
                description: "Optimiza cada pieza de contenido para SEO: meta tags, \
                              estructura de headings, densidad de keywords y enlaces internos.".into(),
            },
            AgentConfig {
                role: "distributor".into(),
                specialist: "Distribution Agent".into(),
                level: "junior".into(),
                tools: vec!["social_post".into(), "email_send".into(), "scheduler".into()],
                schedule: Some("0 10 * * 1-5".into()),
                description: "Distribuye el contenido publicado en redes sociales, newsletters \
                              y otros canales configurados automaticamente.".into(),
            },
        ],
        setup_steps: vec![
            SetupStep {
                step: 1,
                title: "Temas de Contenido".into(),
                description: "Define los temas principales sobre los que quieres crear contenido \
                              (ej. inteligencia artificial, productividad, marketing).".into(),
                field_type: "text".into(),
                field_key: "content_topics".into(),
                required: true,
            },
            SetupStep {
                step: 2,
                title: "Calendario de Publicacion".into(),
                description: "Selecciona la frecuencia de publicacion: diaria, 3 veces por semana \
                              o semanal.".into(),
                field_type: "select".into(),
                field_key: "publishing_schedule".into(),
                required: true,
            },
            SetupStep {
                step: 3,
                title: "Audiencia Objetivo".into(),
                description: "Describe tu audiencia ideal: perfil demografico, nivel tecnico \
                              e intereses principales.".into(),
                field_type: "text".into(),
                field_key: "target_audience".into(),
                required: true,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// Finance Team
// ---------------------------------------------------------------------------
fn finance_team() -> TeamTemplate {
    TeamTemplate {
        id: "finance".into(),
        name: "Equipo de Finanzas".into(),
        description: "Automatiza tu contabilidad: procesamiento de facturas, categorizacion de gastos, \
                       generacion de reportes, preparacion fiscal y recordatorios de pago.".into(),
        icon: "calculator".into(),
        category: "finanzas".into(),
        connectors_required: vec!["email".into(), "calendar".into()],
        agents: vec![
            AgentConfig {
                role: "invoices".into(),
                specialist: "Invoice Processor".into(),
                level: "mid".into(),
                tools: vec!["email_read".into(), "ocr_extract".into(), "data_store".into()],
                schedule: Some("0 8 * * 1-5".into()),
                description: "Procesa facturas recibidas por email automaticamente: extrae datos, \
                              valida montos y registra en el sistema contable.".into(),
            },
            AgentConfig {
                role: "expenses".into(),
                specialist: "Expense Categorizer".into(),
                level: "junior".into(),
                tools: vec!["classify_intent".into(), "data_store".into(), "analytics_read".into()],
                schedule: Some("0 9 * * 1-5".into()),
                description: "Categoriza gastos automaticamente segun reglas predefinidas, \
                              detecta anomalias y genera alertas de presupuesto.".into(),
            },
            AgentConfig {
                role: "reports".into(),
                specialist: "Report Generator".into(),
                level: "senior".into(),
                tools: vec!["report_generate".into(), "data_export".into(), "analytics_read".into()],
                schedule: Some("0 17 * * 5".into()),
                description: "Genera reportes financieros semanales y mensuales: estado de resultados, \
                              flujo de caja y balance de gastos vs. ingresos.".into(),
            },
            AgentConfig {
                role: "tax".into(),
                specialist: "Tax Preparer".into(),
                level: "senior".into(),
                tools: vec!["data_store".into(), "report_generate".into(), "file_create".into()],
                schedule: Some("0 8 1 * *".into()),
                description: "Prepara documentacion fiscal mensual, calcula impuestos estimados \
                              y organiza comprobantes para la declaracion.".into(),
            },
            AgentConfig {
                role: "payments".into(),
                specialist: "Payment Reminder".into(),
                level: "junior".into(),
                tools: vec!["email_send".into(), "calendar_create".into(), "reminder_set".into()],
                schedule: Some("0 9 * * 1,4".into()),
                description: "Envia recordatorios de pago a clientes con facturas pendientes, \
                              registra pagos recibidos y actualiza el estado de cuentas.".into(),
            },
        ],
        setup_steps: vec![
            SetupStep {
                step: 1,
                title: "Conectar Email de Facturas".into(),
                description: "Conecta la direccion de email donde recibes facturas y comprobantes \
                              de pago (facturacion@tuempresa.com).".into(),
                field_type: "oauth".into(),
                field_key: "invoice_email".into(),
                required: true,
            },
            SetupStep {
                step: 2,
                title: "Ano Fiscal".into(),
                description: "Selecciona cuando inicia tu ano fiscal para calculos contables \
                              y preparacion de impuestos.".into(),
                field_type: "select".into(),
                field_key: "fiscal_year".into(),
                required: true,
            },
            SetupStep {
                step: 3,
                title: "Tasa de Impuestos".into(),
                description: "Ingresa la tasa de impuestos aplicable a tu negocio \
                              para estimaciones fiscales automaticas (ej. 16%, 21%).".into(),
                field_type: "text".into(),
                field_key: "tax_rate".into(),
                required: true,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_templates_returns_five() {
        let templates = all_templates();
        assert_eq!(templates.len(), 5);
    }

    #[test]
    fn each_template_has_agents() {
        for t in all_templates() {
            assert!(!t.agents.is_empty(), "Template {} has no agents", t.id);
            assert!(t.agents.len() >= 5, "Template {} has fewer than 5 agents", t.id);
        }
    }

    #[test]
    fn get_template_by_id() {
        assert!(get_template("marketing").is_some());
        assert!(get_template("sales").is_some());
        assert!(get_template("support").is_some());
        assert!(get_template("content").is_some());
        assert!(get_template("finance").is_some());
        assert!(get_template("nonexistent").is_none());
    }
}
