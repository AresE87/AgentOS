use super::heatmap::HeatmapData;
use super::roi::ROIReport;

pub struct AnalyticsExporter;

impl AnalyticsExporter {
    /// Export ROI report as formatted text (for PDF/printing)
    pub fn export_roi_text(report: &ROIReport) -> String {
        format!(
            "AgentOS ROI Report -- {}\n\
             ===================================\n\n\
             Tasks Completed:       {}\n\
             Time Saved:            {:.0} minutes ({:.1} hours)\n\
             Estimated Manual Cost: ${:.2}\n\
             LLM Cost:              ${:.4}\n\
             Net Savings:           ${:.2}\n\
             ROI:                   {:.0}%\n\n\
             --- Assumptions ---\n\
             Hourly Rate:           ${:.0}/hr\n\
             Avg Time per Task:     {:.0} min\n",
            report.period,
            report.tasks_completed,
            report.total_time_saved_minutes,
            report.total_time_saved_minutes / 60.0,
            report.estimated_manual_cost,
            report.total_llm_cost,
            report.net_savings,
            report.roi_percentage,
            report.hourly_rate,
            report.avg_minutes_per_task,
        )
    }

    /// Export analytics as CSV
    pub fn export_csv(report: &ROIReport) -> String {
        format!(
            "metric,value\n\
             tasks_completed,{}\n\
             time_saved_minutes,{:.1}\n\
             llm_cost,{:.4}\n\
             manual_cost_estimate,{:.2}\n\
             net_savings,{:.2}\n\
             roi_percentage,{:.1}\n",
            report.tasks_completed,
            report.total_time_saved_minutes,
            report.total_llm_cost,
            report.estimated_manual_cost,
            report.net_savings,
            report.roi_percentage,
        )
    }

    /// Export heatmap as CSV
    pub fn export_heatmap_csv(heatmap: &HeatmapData) -> String {
        let mut csv = String::from("day,hour,count\n");
        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (d, row) in heatmap.grid.iter().enumerate() {
            for (h, count) in row.iter().enumerate() {
                csv.push_str(&format!("{},{},{}\n", day_names[d], h, count));
            }
        }
        csv
    }
}
