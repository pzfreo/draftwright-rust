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
