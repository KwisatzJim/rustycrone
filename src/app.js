const target = document.querySelector('#target');
const load = document.querySelector('#load');
const status = document.querySelector('#status');
const contents = document.querySelector('#contents');
const heading = document.querySelector('#heading');
const save = document.querySelector('#save-job');
let loadedText = null;
let pendingEntry = null;
let deletionIndex = null;
let editingIndex = null;

function editableSchedule(job) {
  if (job.schedule === '@reboot') return { type: 'startup' };
  const fields = job.schedule.split(/\s+/);
  if (fields.length !== 5 || fields[3] !== '*') return null;
  const number = (text, maximum) => /^\d+$/.test(text) && Number(text) <= maximum;
  if (fields[2] !== '*') {
    if (!number(fields[2], 31) || Number(fields[2]) < 1 || !number(fields[0], 59) || !number(fields[1], 23) || fields[4] !== '*') return null;
    return { type: 'monthly', monthDay: String(Number(fields[2])), time: `${fields[1].padStart(2, '0')}:${fields[0].padStart(2, '0')}` };
  }
  const interval = fields[0].match(/^\*\/(1|5|10|15|20|30)$/);
  if (interval && fields[1] === '*' && fields[4] === '*') return { type: 'interval', interval: interval[1] };
  if (!number(fields[0], 59)) return null;
  if (fields[1] === '*' && fields[4] === '*') return { type: 'hourly', minute: fields[0] };
  if (!number(fields[1], 23)) return null;
  const clock = `${fields[1].padStart(2, '0')}:${fields[0].padStart(2, '0')}`;
  if (fields[4] === '*') return { type: 'daily', time: clock };
  if (number(fields[4], 6)) return { type: 'weekly', time: clock, day: String(Number(fields[4])) };
  return null;
}

function describeSchedule(expression) {
  const settings = editableSchedule({ schedule: expression });
  if (!settings) return 'Custom cron schedule.';
  switch (settings.type) {
    case 'startup': return 'When the system’s cron service starts, usually during boot. This is not a login task.';
    case 'interval': return settings.interval === '1' ? 'Every minute.'
      : `Every ${settings.interval} minutes, starting at minute 0 of each hour.`;
    case 'hourly': return `Every hour at minute ${Number(settings.minute)}.`;
    case 'daily': return `Daily at ${settings.time}.`;
    case 'weekly': {
      const weekdays = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
      return `Every ${weekdays[Number(settings.day)]} at ${settings.time}.`;
    }
    case 'monthly': return `On day ${settings.monthDay} of each month at ${settings.time}.${Number(settings.monthDay) > 28 ? ' Months without this date are skipped.' : ''}`;
    default: return 'Custom cron schedule.';
  }
}

function clearEditing() {
  if (editingIndex !== null) {
    command.value = '';
    command.setCustomValidity('');
  }
  editingIndex = null;
  pendingEntry = null;
  document.querySelector('#add-heading').textContent = 'Add job';
  document.querySelector('#cancel-edit').hidden = true;
  document.querySelector('#edit-original').hidden = true;
  preview.hidden = true;
}

function jobParts(line) {
  const text = line.trim();
  const special = text.match(/^(@(?:reboot|yearly|annually|monthly|weekly|daily|midnight|hourly))\s+(\S.*)$/);
  if (special) return { schedule: special[1], command: special[2] };
  const regular = text.match(/^([\d*/,-]+\s+[\d*/,-]+\s+[\d*/,-]+\s+[\d*/,-]+\s+[\d*/,-]+)\s+(\S.*)$/);
  return regular ? { schedule: regular[1], command: regular[2] } : null;
}

function renderJobs() {
  deletionIndex = null;
  document.querySelector('#delete-confirmation').hidden = true;
  const list = document.querySelector('#job-list');
  list.replaceChildren();
  if (loadedText === null) return;
  let count = 0;
  loadedText.split('\n').forEach((line, index) => {
    const job = jobParts(line);
    if (!job) return;
    count++;
    const row = document.createElement('div');
    row.className = 'job-row';
    const details = document.createElement('div');
    const scheduleLabel = document.createElement('p');
    scheduleLabel.className = 'schedule-description';
    scheduleLabel.textContent = describeSchedule(job.schedule);
    const expressionLabel = document.createElement('p');
    expressionLabel.className = 'cron-expression';
    expressionLabel.textContent = `Cron: ${job.schedule}`;
    const commandLabel = document.createElement('code');
    commandLabel.textContent = job.command;
    details.append(scheduleLabel, expressionLabel, commandLabel);
    row.append(details);
    if (target.value === 'user' || target.value === 'root') {
      const actions = document.createElement('div');
      actions.className = 'job-actions';
      const edit = document.createElement('button');
      edit.type = 'button';
      edit.textContent = 'Edit…';
      edit.setAttribute('aria-label', `Edit job: ${job.command}`);
      const settings = editableSchedule(job);
      edit.disabled = !settings;
      if (!settings) edit.title = 'Editing supports startup jobs, preset minute intervals, and simple hourly, daily, weekly, and monthly schedules.';
      edit.addEventListener('click', () => {
        clearEditing();
        deletionIndex = null;
        document.querySelector('#delete-confirmation').hidden = true;
        editingIndex = index;
        command.value = job.command;
        command.setCustomValidity('');
        schedule.value = settings.type;
        if (settings.minute !== undefined) minute.value = settings.minute;
        if (settings.interval !== undefined) interval.value = settings.interval;
        if (settings.time !== undefined) time.value = settings.time;
        if (settings.day !== undefined) day.value = settings.day;
        if (settings.monthDay !== undefined) monthDay.value = settings.monthDay;
        schedule.dispatchEvent(new Event('change'));
        document.querySelector('#add-heading').textContent = 'Edit job';
        document.querySelector('#cancel-edit').hidden = false;
        document.querySelector('#edit-original').hidden = false;
        document.querySelector('#original-entry').textContent = line;
        command.focus();
      });
      actions.append(edit);
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = 'Delete…';
      button.setAttribute('aria-label', `Delete job: ${job.command}`);
      button.addEventListener('click', () => {
        clearEditing();
        updateSave();
        deletionIndex = index;
        document.querySelector('#delete-heading').textContent = target.value === 'root' ? 'Delete this root job?' : 'Delete this user job?';
        document.querySelector('#confirm-delete').textContent = target.value === 'root' ? 'Delete root job…' : 'Delete job';
        document.querySelector('#delete-root-help').hidden = target.value !== 'root';
        document.querySelector('#delete-entry').textContent = line;
        document.querySelector('#delete-confirmation').hidden = false;
        document.querySelector('#cancel-delete').focus();
      });
      actions.append(button);
      row.append(actions);
    }
    list.append(row);
  });
  if (!count) {
    const empty = document.createElement('p');
    empty.className = 'help';
    empty.textContent = loadedText ? 'No recognized job rows. Comments, settings, and other lines are available in Full crontab text.' : 'This crontab is empty.';
    list.append(empty);
  }
}

function updateSave() {
  const root = target.value === 'root';
  save.textContent = root ? editingIndex === null ? 'Save job to root crontab…' : 'Save changes to root crontab…'
    : editingIndex === null ? 'Save job to user crontab' : 'Save changes to user crontab';
  save.disabled = loadedText === null || pendingEntry === null;
  document.querySelector('#save-help').textContent = loadedText === null ? 'Load the selected crontab before saving.'
    : root ? 'Saving requests administrator approval. This command will run as root with full system access at the scheduled time. A root-only backup is created first.'
    : 'Save job installs this schedule. Cron can then run the command at the scheduled time.';
}

target.addEventListener('change', () => {
  clearEditing();
  loadedText = null;
  pendingEntry = null;
  renderJobs();
  updateSave();
  document.querySelector('#job-preview').hidden = true;
  heading.textContent = target.value === 'root' ? 'Root’s crontab' : 'Current user’s crontab';
  load.textContent = target.value === 'root' ? 'Load root crontab…' : 'Load crontab';
  document.querySelector('#root-help').hidden = target.value !== 'root';
  contents.textContent = 'No crontab loaded.';
  status.textContent = 'Click Load crontab to read this selection.';
});

const form = document.querySelector('#job-form');
const schedule = document.querySelector('#schedule');
const minute = document.querySelector('#minute');
const interval = document.querySelector('#interval');
const time = document.querySelector('#time');
const day = document.querySelector('#day');
const monthDay = document.querySelector('#month-day');
const command = document.querySelector('#command');
const preview = document.querySelector('#job-preview');

document.querySelector('#cancel-edit').addEventListener('click', () => {
  clearEditing();
  updateSave();
});

schedule.addEventListener('change', () => {
  pendingEntry = null;
  preview.hidden = true;
  updateSave();
  const hourly = schedule.value === 'hourly';
  const minuteInterval = schedule.value === 'interval';
  const weekly = schedule.value === 'weekly';
  const monthly = schedule.value === 'monthly';
  const startup = schedule.value === 'startup';
  document.querySelector('#minute-field').hidden = !hourly;
  document.querySelector('#interval-field').hidden = !minuteInterval;
  document.querySelector('#time-field').hidden = hourly || minuteInterval || startup;
  document.querySelector('#startup-help').hidden = !startup;
  document.querySelector('#day-field').hidden = !weekly;
  document.querySelector('#month-day-field').hidden = !monthly;
  minute.disabled = !hourly;
  interval.disabled = !minuteInterval;
  time.disabled = hourly || minuteInterval || startup;
  day.disabled = !weekly;
  monthDay.disabled = !monthly;
});

form.addEventListener('input', () => {
  pendingEntry = null;
  updateSave();
  preview.hidden = true;
  command.setCustomValidity('');
});

form.addEventListener('submit', (event) => {
  event.preventDefault();
  const jobCommand = command.value.trim();
  if (!jobCommand || /[\r\n\0]/.test(jobCommand)) {
    command.setCustomValidity('Enter a command on one line.');
    command.reportValidity();
    return;
  }
  const owner = target.value === 'root' ? 'Root' : 'Current user';
  let expression;
  if (schedule.value === 'startup') {
    expression = '@reboot';
  } else if (schedule.value === 'interval') {
    expression = `*/${interval.value} * * * *`;
  } else if (schedule.value === 'hourly') {
    expression = `${Number(minute.value)} * * * *`;
  } else {
    const [hour, minutes] = time.value.split(':').map(Number);
    const monthly = schedule.value === 'monthly';
    expression = `${minutes} ${hour} ${monthly ? Number(monthDay.value) : '*'} * ${schedule.value === 'weekly' ? day.value : '*'}`;
  }
  document.querySelector('#preview-heading').textContent = `${owner} — ${editingIndex === null ? 'job preview' : 'replacement preview'}`;
  document.querySelector('#preview-description').textContent = describeSchedule(expression);
  document.querySelector('#preview-entry').textContent = `${expression} ${jobCommand}`;
  pendingEntry = `${expression} ${jobCommand}`;
  updateSave();
  preview.hidden = false;
});

load.addEventListener('click', async () => {
  clearEditing();
  loadedText = null;
  renderJobs();
  updateSave();
  load.disabled = true;
  target.disabled = true;
  status.textContent = target.value === 'root' ? 'Waiting for system administrator authorization…' : 'Reading crontab…';
  contents.textContent = '';
  try {
    const text = await window.__TAURI__.core.invoke('read_crontab', { target: target.value });
    loadedText = text;
    contents.textContent = text || 'This crontab is empty.';
    status.textContent = text ? 'Crontab loaded. No changes have been made.' : 'No scheduled jobs found.';
  } catch (error) {
    status.textContent = 'Unable to load crontab.';
    contents.textContent = String(error);
  } finally {
    load.disabled = false;
    target.disabled = false;
    updateSave();
    renderJobs();
  }
});

save.addEventListener('click', async () => {
  if (save.disabled) return;
  const entry = pendingEntry;
  const original = loadedText;
  const lineIndex = editingIndex;
  const root = target.value === 'root';
  document.querySelector('#delete-confirmation').hidden = true;
  deletionIndex = null;
  const controls = [...document.querySelectorAll('button, input, select')];
  const disabledStates = controls.map(control => control.disabled);
  controls.forEach(control => { control.disabled = true; });
  status.textContent = root ? 'Waiting for administrator approval to back up and save root’s crontab…' : 'Backing up and saving your user crontab…';
  try {
    const operation = root ? (lineIndex === null ? 'save_root_job' : 'edit_root_job')
      : (lineIndex === null ? 'save_user_job' : 'edit_user_job');
    const backup = await window.__TAURI__.core.invoke(operation,
      lineIndex === null ? { expected: original, entry } : { expected: original, lineIndex, entry });
    if (lineIndex === null) {
      loadedText = original + (original && !original.endsWith('\n') ? '\n' : '') + entry + '\n';
    } else {
      const lines = original.match(/[^\n]*\n|[^\n]+$/g) || [];
      lines[lineIndex] = entry + (lines[lineIndex].endsWith('\r\n') ? '\r\n' : '\n');
      loadedText = lines.join('');
    }
    contents.textContent = loadedText;
    pendingEntry = null;
    preview.hidden = true;
    clearEditing();
    status.textContent = `${lineIndex === null ? 'Job saved' : 'Job updated'} and verified. Backup: ${backup}`;
  } catch (error) {
    loadedText = null;
    clearEditing();
    status.textContent = `${error} Load your crontab again before another save.`;
  } finally {
    controls.forEach((control, index) => { control.disabled = disabledStates[index]; });
    updateSave();
    renderJobs();
  }
});

document.querySelector('#cancel-delete').addEventListener('click', () => {
  deletionIndex = null;
  document.querySelector('#delete-confirmation').hidden = true;
});

document.querySelector('#confirm-delete').addEventListener('click', async () => {
  if (loadedText === null || deletionIndex === null) return;
  const original = loadedText;
  const lineIndex = deletionIndex;
  const root = target.value === 'root';
  deletionIndex = null;
  const controls = [...document.querySelectorAll('button, input, select')];
  const disabledStates = controls.map(control => control.disabled);
  controls.forEach(control => { control.disabled = true; });
  status.textContent = root ? 'Waiting for administrator approval to back up and delete the selected root job…' : 'Backing up and deleting the selected job…';
  try {
    const backup = await window.__TAURI__.core.invoke(root ? 'delete_root_job' : 'delete_user_job', { expected: original, lineIndex });
    const lines = original.match(/[^\n]*\n|[^\n]+$/g) || [];
    lines.splice(lineIndex, 1);
    loadedText = lines.join('');
    contents.textContent = loadedText || 'This crontab is empty.';
    status.textContent = `Job deleted and verified. Backup: ${backup}`;
  } catch (error) {
    loadedText = null;
    status.textContent = `${error} Load your crontab again before making changes.`;
  } finally {
    controls.forEach((control, index) => { control.disabled = disabledStates[index]; });
    updateSave();
    renderJobs();
  }
});
