// The success conditions of the MCP evaluation's tasks (tasks.json), checked on a task's
// resulting Spec and the Plan made from it, with the part's default Plan as the baseline.
// Every condition reports pass or fail with the reason; a task succeeds when all pass.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('.', import.meta.url));

export function loadTasks() {
  return JSON.parse(readFileSync(resolve(root, 'tasks.json'), 'utf8'));
}

/** A dimension ID's quantity: `d:<feature>/<parameter>[@<view>]` without `d:` and the view. */
export const quantity = id => id.replace(/^d:/, '').replace(/@[^@/]*$/, '');

const planned = (plan, q) => plan.dimensions.filter(d => quantity(d.id) === q);
const viewName = view => String(view ?? '').replace(/^v:/, '');

/** The Spec's entry for quantity *q*, whichever ID form (with or without a view) keys it. */
function specEntry(spec, q) {
  const entries = Object.entries(spec.dimensions ?? {}).filter(([id]) => quantity(id) === q);
  return entries.length ? entries[0][1] : undefined;
}

/** Whether a problem is about quantity or feature *about*: a subject naming it. */
function concerns(problem, about) {
  return (problem.subjects ?? []).some(subject => subject === about || subject.startsWith(`d:${about}/`)
    || (subject.startsWith('d:') && quantity(subject) === about));
}

const CHECKS = {
  sheet(c, { plan }) {
    const wrong = [];
    if (c.size && plan.sheet.size !== c.size) wrong.push(`size is ${plan.sheet.size}, not ${c.size}`);
    if (c.scale && plan.sheet.scale !== c.scale) wrong.push(`scale is ${plan.sheet.scale}, not ${c.scale}`);
    for (const [field, value] of Object.entries(c.title ?? {})) {
      const got = plan.sheet.title?.[field];
      if (got !== value) wrong.push(`title ${field} is ${JSON.stringify(got)}, not ${JSON.stringify(value)}`);
    }
    return wrong;
  },
  view_absent: (c, { plan }) => plan.views.some(v => v.id === c.view) ? [`${c.view} is drawn`] : [],
  view_present: (c, { plan }) => plan.views.some(v => v.id === c.view) ? [] : [`${c.view} is not drawn`],
  dimension(c, { plan }) {
    const found = planned(plan, c.quantity);
    if (c.planned === false) return found.length ? [`${c.quantity} is drawn (${found.map(d => d.id).join(', ')})`] : [];
    if (!found.length) return [`${c.quantity} is not drawn`];
    const wrong = [];
    for (const d of found) {
      if (c.view && d.view !== c.view) wrong.push(`${d.id} is in ${d.view}, not ${c.view}`);
      if (c.label_includes && !String(d.label).includes(c.label_includes)) wrong.push(`${d.id}'s label ${JSON.stringify(d.label)} lacks ${JSON.stringify(c.label_includes)}`);
      if (c.right_of_view_centre) {
        const view = plan.views.find(v => v.id === d.view);
        if (!d.placed || !view || d.placed.label_box[0] < view.at[0]) wrong.push(`${d.id}'s label is not right of ${d.view}'s centre`);
      }
    }
    return wrong;
  },
  placed(c, { spec }) {
    const entry = specEntry(spec, c.quantity);
    if (!entry) return [`the Spec has no entry for ${c.quantity}`];
    const place = entry.place ?? {};
    const wrong = [];
    if (c.view && viewName(place.view) !== viewName(c.view)) wrong.push(`${c.quantity}'s place.view is ${JSON.stringify(place.view)}, not ${c.view}`);
    if (c.side && place.side !== c.side) wrong.push(`${c.quantity}'s place.side is ${JSON.stringify(place.side)}, not ${c.side}`);
    if (c.pinned !== undefined && Boolean(place.pinned) !== c.pinned) wrong.push(`${c.quantity}'s place.pinned is ${JSON.stringify(place.pinned)}`);
    return wrong;
  },
  kept_others(c, { plan, baseline }) {
    const now = new Set(plan.dimensions.map(d => quantity(d.id)));
    return [...new Set(baseline.dimensions.map(d => quantity(d.id)))]
      .filter(q => !c.except.includes(q) && !now.has(q)).map(q => `${q} is no longer drawn`);
  },
  no_problems(c, { plan }) {
    return plan.problems.filter(p => c.about.some(about => concerns(p, about)))
      .map(p => `problem ${p.code ?? p.id}: ${p.message}`);
  },
};

/** Checks *task* on `{ spec, plan, baseline }`: `{ success, conditions: [{ condition, pass, why }] }`. */
export function check(task, documents) {
  const conditions = task.success.map(condition => {
    const run = CHECKS[condition.type];
    if (!run) throw new Error(`${task.id}: unknown condition ${condition.type}`);
    const why = run(condition, documents);
    return { condition, pass: why.length === 0, why };
  });
  return { success: conditions.every(c => c.pass), conditions };
}
