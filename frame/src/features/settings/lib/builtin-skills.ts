import * as m from "$shared/i18n/messages";

/** The built-in skills as a person reads them. Their files name and describe
 *  them in English for the model; this page shows the interface language. A
 *  skill the person wrote keeps its own name and description. */
const BUILTIN: Record<string, { title: () => string; line: () => string }> = {
  "campaign-plan": { title: m.work_flow_campaign, line: m.work_flow_campaign_line },
  "compare-and-choose": { title: m.skill_compare, line: m.skill_compare_line },
  "competitor-landscape": { title: m.work_flow_landscape, line: m.work_flow_landscape_line },
  "exam-prep": { title: m.work_flow_exam, line: m.work_flow_exam_line },
  "explain-a-subject": { title: m.skill_explain_subject, line: m.skill_explain_subject_line },
  "explain-code": { title: m.work_flow_explain_code, line: m.work_flow_explain_code_line },
  "fix-a-bug": { title: m.work_flow_fix_test, line: m.work_flow_fix_test_line },
  "inbox-triage": { title: m.work_flow_inbox, line: m.work_flow_inbox_line },
  "investor-list": { title: m.work_flow_investors, line: m.work_flow_investors_line },
  "job-search": { title: m.skill_job_search, line: m.skill_job_search_line },
  "learning-path": { title: m.work_flow_learn, line: m.work_flow_learn_line },
  "map-a-project": { title: m.work_flow_map_project, line: m.work_flow_map_project_line },
  "market-map": { title: m.work_flow_market, line: m.work_flow_market_line },
  "meeting-prep": { title: m.work_flow_meeting, line: m.work_flow_meeting_line },
  moodboard: { title: m.work_flow_moodboard, line: m.work_flow_moodboard_line },
  "plan-my-day": { title: m.work_flow_day, line: m.work_flow_day_line },
  "pricing-research": { title: m.work_flow_pricing, line: m.work_flow_pricing_line },
  research: { title: m.skill_research, line: m.skill_research_line },
  "review-a-change": { title: m.work_flow_review_change, line: m.work_flow_review_change_line },
  "show-what-you-can-do": { title: m.skill_show, line: m.skill_show_line },
  "system-design": { title: m.skill_system_design, line: m.skill_system_design_line },
  "today-from-my-messages": { title: m.skill_today, line: m.skill_today_line },
  "trip-planning": { title: m.skill_trip, line: m.skill_trip_line },
  "ui-teardown": { title: m.work_flow_teardown, line: m.work_flow_teardown_line },
  "weekly-status": { title: m.work_flow_status, line: m.work_flow_status_line },
};

/** A built-in skill's title and one-line description in the interface
 *  language, when it is one this build ships. */
export function builtinSkill(name: string, builtin: boolean) {
  const known = builtin ? BUILTIN[name] : undefined;
  return known ? { title: known.title(), line: known.line() } : null;
}
