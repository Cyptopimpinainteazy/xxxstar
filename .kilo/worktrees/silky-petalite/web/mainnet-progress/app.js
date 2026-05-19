async function loadProgress() {
  const res = await fetch("data/mainnet_progress.json", { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`Failed to load progress data (${res.status})`);
  }
  return res.json();
}

async function loadGoals() {
  const res = await fetch("data/mainnet_goals.json", { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`Failed to load goals data (${res.status})`);
  }
  return res.json();
}

function rowHtml(g) {
  return `
    <tr>
      <td>${g.id}</td>
      <td>${g.name}</td>
      <td><span class="status ${g.status}">${g.status}</span></td>
      <td>${g.score.toFixed(1)}</td>
      <td>${g.piecePercent.toFixed(1)}%</td>
    </tr>
  `;
}

async function main() {
  try {
    const [data, goals] = await Promise.all([loadProgress(), loadGoals()]);

    document.getElementById("meta").textContent = `Generated ${data.generatedDate} from ${data.source}`;
    document.getElementById("overall").textContent = `${data.overallPercent.toFixed(1)}%`;
    document.getElementById("remaining").textContent = `Remaining: ${data.remainingPercent.toFixed(1)}%`;

    document.getElementById("green").textContent = data.gateCounts.green;
    document.getElementById("yellow").textContent = data.gateCounts.yellow;
    document.getElementById("red").textContent = data.gateCounts.red;

    document.getElementById("gateRows").innerHTML = data.gates.map(rowHtml).join("");

    document.getElementById("goalsOverall").textContent = `${goals.overall.percent.toFixed(1)}%`;
    document.getElementById("goalsCounts").textContent = `Done: ${goals.overall.done} | Open: ${goals.overall.todo} | Exempt: ${goals.overall.exempt}`;
    document.getElementById("goalRows").innerHTML = goals.files
      .map(
        (f) => `
      <tr>
        <td>${f.file}</td>
        <td>${f.done}</td>
        <td>${f.todo}</td>
        <td>${f.exempt}</td>
        <td>${f.percent.toFixed(1)}%</td>
      </tr>
    `
      )
      .join("");
  } catch (err) {
    document.getElementById("meta").textContent = `Error: ${err.message}`;
  }
}

main();
