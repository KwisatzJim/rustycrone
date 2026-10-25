import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

// Exercise the page's event handlers without opening a GUI or touching real crontabs.
function page(table = '') {
  const nodes = new Map();
  const element = () => ({
    value: '', hidden: true, disabled: false, children: [], listeners: {},
    addEventListener(type, handler) { this.listeners[type] = handler; },
    dispatchEvent(event) { this.listeners[event.type]?.(event); },
    append(...children) { this.children.push(...children); },
    replaceChildren() { this.children = []; },
    setAttribute() {}, setCustomValidity() {}, focus() {},
  });
  const get = selector => {
    if (!nodes.has(selector)) nodes.set(selector, element());
    return nodes.get(selector);
  };
  const calls = [];
  const state = { table, failure: null };
  vm.runInNewContext(fs.readFileSync(new URL('../src/app.js', import.meta.url), 'utf8'), {
    Event,
    document: { querySelector: get, querySelectorAll: () => [...nodes.values()], createElement: element },
    window: { __TAURI__: { core: { invoke: async (name, args) => {
      calls.push({ name, args });
      if (state.failure) throw state.failure;
      return name === 'read_crontab' ? state.table : '/private/backup.crontab';
    } } } },
  });
  get('#target').value = 'user';
  get('#command').value = '/usr/bin/true';
  get('#time').value = '09:30';
  get('#minute').value = '0';
  get('#day').value = '1';
  get('#month-day').value = '1';
  get('#interval').value = '15';
  const click = selector => get(selector).listeners.click();
  const preview = choice => {
    get('#schedule').value = choice;
    get('#schedule').dispatchEvent(new Event('change'));
    get('#job-form').listeners.submit({ preventDefault() {} });
  };
  const actions = index => get('#job-list').children[index].children[1].children;
  return { get, calls, state, click, preview, actions };
}

test('all schedule presets produce the intended expression and descriptions', () => {
  const p = page();
  for (const [choice, expression] of [
    ['hourly', '0 * * * *'], ['interval', '*/15 * * * *'], ['daily', '30 9 * * *'],
    ['weekly', '30 9 * * 1'], ['monthly', '30 9 1 * *'], ['startup', '@reboot'],
  ]) {
    p.preview(choice);
    assert.equal(p.get('#preview-entry').textContent, `${expression} /usr/bin/true`);
    assert.equal(p.get('#save-job').disabled, true, 'loading is required before saving');
  }
  for (const value of ['1', '5', '10', '15', '20', '30']) {
    p.get('#interval').value = value;
    p.preview('interval');
    assert.equal(p.get('#preview-entry').textContent, `*/${value} * * * * /usr/bin/true`);
  }
  p.get('#month-day').value = '31';
  p.preview('monthly');
  assert.match(p.get('#preview-description').textContent, /Months without this date are skipped/);
  p.preview('startup');
  assert.equal(p.get('#time').disabled, true);
  assert.match(p.get('#preview-description').textContent, /cron service starts/);
});

for (const owner of ['user', 'root']) {
  test(`${owner} edits preserve settings and delete only the selected duplicate`, async () => {
    const original = '# Keep\nSHELL=/bin/sh\n0 * * * * /same\n0 * * * * /same\n';
    const p = page(original);
    p.get('#target').value = owner;
    p.get('#target').dispatchEvent(new Event('change'));
    await p.click('#load');
    p.actions(1)[0].listeners.click();
    p.preview('daily');
    await p.click('#save-job');
    assert.equal(p.calls.at(-1).name, `edit_${owner}_job`);
    assert.equal(p.calls.at(-1).args.lineIndex, 3);
    assert.equal(p.calls.at(-1).args.expected, original);
    const edited = '# Keep\nSHELL=/bin/sh\n0 * * * * /same\n30 9 * * * /same\n';
    assert.equal(p.get('#contents').textContent, edited);
    p.actions(0)[1].listeners.click();
    const count = p.calls.length;
    p.click('#cancel-delete');
    await p.click('#confirm-delete');
    assert.equal(p.calls.length, count);
    p.actions(0)[1].listeners.click();
    await p.click('#confirm-delete');
    assert.equal(p.calls.at(-1).name, `delete_${owner}_job`);
    assert.equal(p.calls.at(-1).args.lineIndex, 2);
    assert.equal(p.get('#contents').textContent, '# Keep\nSHELL=/bin/sh\n30 9 * * * /same\n');
  });
}

test('failed authorization requires reload before another write', async () => {
  const p = page('0 * * * * /usr/bin/true\n');
  p.get('#target').value = 'root';
  await p.click('#load');
  p.actions(0)[1].listeners.click();
  p.state.failure = 'Administrator authorization was cancelled.';
  await p.click('#confirm-delete');
  assert.match(p.get('#status').textContent, /cancelled/);
  p.preview('hourly');
  assert.equal(p.get('#save-job').disabled, true);
});

test('custom schedules retain their exact expression and cannot be simplified by editing', async () => {
  const p = page('0 9 * * 1-5 /usr/bin/true\n');
  await p.click('#load');
  const details = p.get('#job-list').children[0].children[0].children;
  assert.equal(details[0].textContent, 'Custom cron schedule.');
  assert.equal(details[1].textContent, 'Cron: 0 9 * * 1-5');
  assert.equal(p.actions(0)[0].disabled, true);
});
