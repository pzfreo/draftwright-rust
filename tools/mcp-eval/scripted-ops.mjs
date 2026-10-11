const segment = id => id.replaceAll('~', '~0').replaceAll('/', '~1');
const set = (path, value) => ({ op: 'set', path, value });

export function pinCallouts(task, items) {
  const required = task.success.filter(condition => condition.type === 'placed');
  return required.map((condition, i) => {
    const id = 'd:' + condition.quantity;
    if (!items.some(item => item.id === id)) throw new Error(`summary missing required callout ${id}`);
    return set(`dimensions/${segment(id)}/place`, {
      view: condition.view, side: condition.side, lane: i,
      along: ['start', 'middle', 'end'][i], pinned: condition.pinned,
    });
  });
}

export async function keepIntent(apply, id) {
  await apply([set(`dimensions/${segment(id)}/tolerance`, '±0.02'), set('views/iso/omit', true)]);
  await apply([set('sheet/size', 'A3'), set('sheet/scale', '5:1')]);
}

// A deliberate scripted editor action for report-only extension warnings, not a planner fix.
export function oppositeExtension(item, line) {
  if (item.placement?.kind !== 'dimension') throw new Error('extension has no dimension geometry');
  const p = item.placement.geometry, shaft = p.lines[line];
  if (!shaft) throw new Error('extension line is missing');
  const horizontal = Math.abs(p.rotation % 180) < 1e-6;
  const k = horizontal ? 1 : 0;
  const side = shaft[1][k] > shaft[0][k]
    ? (horizontal ? 'below' : 'left') : (horizontal ? 'above' : 'right');
  return set(`dimensions/${segment(item.item.id)}/place`, {
    view: item.item.view.replace(/^v:/, ''), side, lane: 0,
  });
}
