# Industry Verticals Pro — v4.1.0

AgentOS v4.1.0 introduces nine industry-specific vertical modules, each with domain-tailored data models, business logic, and IPC commands accessible from both the Tauri backend and the React frontend.

## Modules

### R131 — Legal Suite (`verticals/legal.rs`)
- **LegalCase**: case number, title, client, status, documents, notes
- **Operations**: create_case, list_cases, search_cases, analyze_document
- **IPC**: `cmd_legal_create_case`, `cmd_legal_list_cases`, `cmd_legal_search`, `cmd_legal_analyze`

### R132 — Medical Assistant (`verticals/medical.rs`)
- **PatientRecord**: name, DOB, conditions, medications, clinical notes
- **Operations**: add_record, search_records, drug_interaction_check, summarize_history
- **IPC**: `cmd_medical_add`, `cmd_medical_search`, `cmd_medical_interactions`, `cmd_medical_summary`

### R133 — Accounting Engine (`verticals/accounting.rs`)
- **Transaction**: date, description, amount, category, account, type (income/expense/transfer)
- **Operations**: add_transaction, get_balance, generate_report, categorize_transaction
- **IPC**: `cmd_accounting_add`, `cmd_accounting_balance`, `cmd_accounting_report`, `cmd_accounting_categorize`

### R134 — Real Estate Agent (`verticals/real_estate.rs`)
- **Property**: address, price, bedrooms, bathrooms, sqft, status, type
- **Operations**: add_property, search_properties, calculate_roi, generate_listing
- **IPC**: `cmd_realestate_add`, `cmd_realestate_search`, `cmd_realestate_roi`, `cmd_realestate_listing`

### R135 — Education Assistant (`verticals/education.rs`)
- **Course**: title, subject, level, lessons with ordering
- **StudentProgress**: completed lessons, quiz scores, overall grade
- **Operations**: create_course, generate_quiz, grade_answer, track_progress
- **IPC**: `cmd_edu_create_course`, `cmd_edu_quiz`, `cmd_edu_grade`, `cmd_edu_progress`

### R136 — HR Manager (`verticals/hr.rs`)
- **Employee**: name, department, role, hire date, status, salary
- **Operations**: add_employee, list_employees, generate_offer_letter, calculate_benefits
- **IPC**: `cmd_hr_add`, `cmd_hr_list`, `cmd_hr_offer_letter`, `cmd_hr_benefits`

### R137 — Supply Chain Manager (`verticals/supply_chain.rs`)
- **Shipment**: origin, destination, status, carrier, ETA, weight, items
- **Operations**: track_shipment, optimize_route, forecast_demand, list_shipments
- **IPC**: `cmd_supply_track`, `cmd_supply_optimize`, `cmd_supply_forecast`, `cmd_supply_list`

### R138 — Construction Manager (`verticals/construction.rs`)
- **ConstructionProject**: name, site, budget, timeline, milestones
- **Operations**: create_project, update_milestone, calculate_budget, safety_checklist
- **IPC**: `cmd_construction_create`, `cmd_construction_milestone`, `cmd_construction_budget`, `cmd_construction_safety`

### R139 — Agriculture Assistant (`verticals/agriculture.rs`)
- **CropPlan**: crop, field, acres, planted date, expected harvest, status
- **Operations**: create_plan, weather_impact, irrigation_schedule, yield_forecast
- **IPC**: `cmd_agri_create_plan`, `cmd_agri_weather`, `cmd_agri_irrigation`, `cmd_agri_yield`

## Frontend Usage

All hooks are available via `useAgent()`:

```typescript
const {
  // Legal
  legalCreateCase, legalListCases, legalSearch, legalAnalyze,
  // Medical
  medicalAdd, medicalSearch, medicalInteractions, medicalSummary,
  // Accounting
  accountingAdd, accountingBalance, accountingReport, accountingCategorize,
  // Real Estate
  realestateAdd, realestateSearch, realestateRoi, realestateListing,
  // Education
  eduCreateCourse, eduQuiz, eduGrade, eduProgress,
  // HR
  hrAdd, hrList, hrOfferLetter, hrBenefits,
  // Supply Chain
  supplyTrack, supplyOptimize, supplyForecast, supplyList,
  // Construction
  constructionCreate, constructionMilestone, constructionBudget, constructionSafety,
  // Agriculture
  agriCreatePlan, agriWeather, agriIrrigation, agriYield,
} = useAgent();
```

## Architecture

Each vertical module follows a consistent pattern:
1. A domain-specific data struct (serializable via serde)
2. A manager/assistant/engine struct with in-memory storage
3. CRUD and domain-specific analytics operations
4. 4 Tauri IPC commands per module (36 total)
5. Corresponding TypeScript hooks in `useAgent.ts`
