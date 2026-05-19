import React, { useEffect, useMemo, useState } from 'react';
import clsx from 'clsx';
import {
  shellAlerts,
  shellFlowCards,
  shellMilestones,
  shellRouteGroups
} from '../mock/frontendShellData';
import { getRouteContract } from '../mock/frontendShellContractData';

function findRoute(routeId) {
  for (const group of shellRouteGroups) {
    const route = group.routes.find((candidate) => candidate.id === routeId);
    if (route) {
      return {
        ...route,
        groupLabel: group.label,
        groupSummary: group.summary
      };
    }
  }

  const fallbackGroup = shellRouteGroups[0];
  return {
    ...fallbackGroup.routes[0],
    groupLabel: fallbackGroup.label,
    groupSummary: fallbackGroup.summary
  };
}

export default function FrontendShell() {
  const [activeRouteId, setActiveRouteId] = useState(shellRouteGroups[0].routes[0].id);
  const [activeScreenId, setActiveScreenId] = useState(shellRouteGroups[0].routes[0].screens[0].id);
  const [activeScenarioId, setActiveScenarioId] = useState(shellRouteGroups[0].routes[0].scenarios[0].id);
  const [activeJourneyStep, setActiveJourneyStep] = useState(0);

  const activeRoute = useMemo(() => findRoute(activeRouteId), [activeRouteId]);
  const activeRouteContract = useMemo(() => getRouteContract(activeRouteId), [activeRouteId]);

  useEffect(() => {
    setActiveScreenId(activeRoute.screens[0].id);
    setActiveScenarioId(activeRoute.scenarios[0].id);
    setActiveJourneyStep(0);
  }, [activeRoute]);

  const activeScreen = useMemo(
    () => activeRoute.screens.find((screen) => screen.id === activeScreenId) ?? activeRoute.screens[0],
    [activeRoute, activeScreenId]
  );

  const activeScenario = useMemo(
    () => activeRoute.scenarios.find((scenario) => scenario.id === activeScenarioId) ?? activeRoute.scenarios[0],
    [activeRoute, activeScenarioId]
  );

  const highlightedJourney = activeRoute.journey[activeJourneyStep] ?? activeRoute.journey[0];

  return (
    <main className="min-h-screen bg-[#0b0911] text-[#f4efe5]">
      <div className="mx-auto flex min-h-screen max-w-[1600px] flex-col gap-8 px-5 py-6 lg:px-8">
        <header className="overflow-hidden rounded-[32px] border border-white/10 bg-[radial-gradient(circle_at_top_left,_rgba(244,151,72,0.22),_transparent_38%),linear-gradient(135deg,_rgba(23,17,27,0.98),_rgba(11,9,17,1))] p-6 shadow-[0_24px_80px_rgba(0,0,0,0.45)] lg:p-8">
          <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-3xl space-y-4">
              <div className="inline-flex items-center gap-2 rounded-full border border-[#f49748]/35 bg-[#f49748]/10 px-3 py-1 text-[11px] uppercase tracking-[0.28em] text-[#ffd2ab]">
                Mock Contract Mode
              </div>
              <div className="space-y-3">
                <p className="text-sm uppercase tracking-[0.35em] text-[#9f9487]">Disposable frontend shell</p>
                <h1 className="max-w-2xl text-4xl font-semibold leading-tight text-[#fbf6ef] lg:text-6xl">
                  X3 can rehearse route architecture now without binding itself to unstable protocol contracts.
                </h1>
                <p className="max-w-2xl text-base leading-7 text-[#c5b7a7] lg:text-lg">
                  This surface stays local, fixture-backed, and intentionally disposable. It exists to refine information architecture, copy boundaries,
                  route density, and interaction pacing while backend freeze work closes the real contract set.
                </p>
              </div>
            </div>
            <div className="grid gap-3 rounded-[28px] border border-white/8 bg-black/20 p-4 backdrop-blur md:grid-cols-2 lg:min-w-[420px]">
              {shellMilestones.map((milestone) => (
                <div key={milestone.gate} className="rounded-[22px] border border-white/8 bg-white/[0.03] p-4">
                  <p className="text-[11px] uppercase tracking-[0.28em] text-[#9f9487]">{milestone.status}</p>
                  <h2 className="mt-2 text-base font-medium text-[#fbf6ef]">{milestone.gate}</h2>
                  <p className="mt-2 text-sm leading-6 text-[#b7aa9d]">{milestone.note}</p>
                </div>
              ))}
            </div>
          </div>
        </header>

        <section className="grid gap-6 lg:grid-cols-[320px_minmax(0,1fr)] xl:grid-cols-[360px_minmax(0,1fr)]">
          <aside className="rounded-[28px] border border-white/8 bg-[#14111a] p-4 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-5">
            <div className="mb-5 flex items-center justify-between gap-3">
              <div>
                <p className="text-[11px] uppercase tracking-[0.28em] text-[#9f9487]">Route map</p>
                <h2 className="mt-2 text-xl font-medium text-[#fbf6ef]">Frontend shells</h2>
              </div>
              <div className="rounded-full border border-[#5bc0be]/25 bg-[#5bc0be]/10 px-3 py-1 text-xs uppercase tracking-[0.22em] text-[#a8f2ec]">
                fixture only
              </div>
            </div>

            <div className="space-y-5">
              {shellRouteGroups.map((group) => (
                <section key={group.id} className="space-y-3">
                  <div>
                    <h3 className="text-sm font-semibold uppercase tracking-[0.22em] text-[#f1b780]">{group.label}</h3>
                    <p className="mt-1 text-sm leading-6 text-[#9f9487]">{group.summary}</p>
                  </div>

                  <div className="space-y-2">
                    {group.routes.map((route) => {
                      const isActive = route.id === activeRouteId;
                      const routeContract = getRouteContract(route.id);

                      return (
                        <button
                          key={route.id}
                          type="button"
                          onClick={() => setActiveRouteId(route.id)}
                          className={clsx(
                            'w-full rounded-[22px] border px-4 py-4 text-left transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]',
                            isActive
                              ? 'border-[#f49748]/45 bg-[#f49748]/10 shadow-[0_12px_30px_rgba(244,151,72,0.12)]'
                              : 'border-white/7 bg-white/[0.02] hover:border-white/18 hover:bg-white/[0.04]'
                          )}
                        >
                          <div className="flex items-start justify-between gap-3">
                            <div>
                              <p className="text-sm font-medium text-[#fbf6ef]">{route.label}</p>
                              <p className="mt-1 text-xs uppercase tracking-[0.22em] text-[#9f9487]">{route.readiness}</p>
                              <p className="mt-2 text-[11px] uppercase tracking-[0.2em] text-[#78d7d1]">
                                {routeContract.directReadCount > 0
                                  ? `${routeContract.directReadCount} direct-read method${routeContract.directReadCount === 1 ? '' : 's'}`
                                  : 'sidecar-only route'}
                              </p>
                            </div>
                            <span
                              className={clsx(
                                'rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.24em]',
                                route.stage === 'blocked'
                                  ? 'bg-[#59252f] text-[#ffc2d1]'
                                  : 'bg-[#173f36] text-[#aef3db]'
                              )}
                            >
                              {route.stage}
                            </span>
                          </div>
                          <p className="mt-3 text-sm leading-6 text-[#c5b7a7]">{route.description}</p>
                        </button>
                      );
                    })}
                  </div>
                </section>
              ))}
            </div>
          </aside>

          <div className="grid gap-6">
            <section className="rounded-[28px] border border-white/8 bg-[#15111b] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6">
              <div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
                <div className="max-w-3xl">
                  <p className="text-[11px] uppercase tracking-[0.28em] text-[#9f9487]">{activeRoute.groupLabel}</p>
                  <h2 className="mt-3 text-3xl font-semibold text-[#fbf6ef]">{activeRoute.label}</h2>
                  <p className="mt-3 max-w-2xl text-base leading-7 text-[#c5b7a7]">{activeRoute.description}</p>
                  <p className="mt-4 text-sm leading-6 text-[#9f9487]">{activeRoute.groupSummary}</p>
                </div>
                <div className="rounded-[24px] border border-white/8 bg-black/20 px-4 py-4 lg:max-w-[320px]">
                  <p className="text-[11px] uppercase tracking-[0.28em] text-[#9f9487]">Scenario state</p>
                  <p className="mt-3 text-sm leading-6 text-[#f3e7da]">{activeScenario.description}</p>
                  <div className="mt-4 flex flex-wrap gap-2">
                    {activeRoute.scenarios.map((scenario) => (
                      <button
                        key={scenario.id}
                        type="button"
                        onClick={() => setActiveScenarioId(scenario.id)}
                        className={clsx(
                          'rounded-full border px-3 py-2 text-[11px] uppercase tracking-[0.24em] transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]',
                          scenario.id === activeScenarioId
                            ? 'border-[#f49748]/45 bg-[#f49748]/12 text-[#fff3e1]'
                            : 'border-white/10 bg-white/[0.02] text-[#b7aa9d] hover:border-white/20 hover:text-[#fbf6ef]'
                        )}
                      >
                        {scenario.label}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              <div className="mt-6 grid gap-4 md:grid-cols-3">
                {activeRoute.metrics.map((metric) => (
                  <article key={metric.label} className="rounded-[24px] border border-white/7 bg-white/[0.03] p-4">
                    <p className="text-[11px] uppercase tracking-[0.24em] text-[#9f9487]">{metric.label}</p>
                    <p className="mt-3 text-2xl font-semibold text-[#fbf6ef]">{metric.value}</p>
                  </article>
                ))}
              </div>
            </section>

            <section className="grid gap-6 xl:grid-cols-[1.25fr_0.75fr]">
              <article className="rounded-[28px] border border-white/8 bg-[linear-gradient(180deg,_rgba(255,255,255,0.04),_rgba(255,255,255,0.015))] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-[11px] uppercase tracking-[0.28em] text-[#9f9487]">Route experience</p>
                    <h3 className="mt-2 text-2xl font-semibold text-[#fbf6ef]">{activeScreen.headline}</h3>
                    <p className="mt-3 max-w-3xl text-sm leading-7 text-[#c5b7a7]">{activeScreen.summary}</p>
                  </div>
                  <span className="rounded-full border border-[#e8d4aa]/30 bg-[#e8d4aa]/10 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-[#ffe9b8]">
                    {activeScreen.kicker}
                  </span>
                </div>

                <div className="mt-6 flex flex-wrap gap-2">
                  {activeRoute.screens.map((screen) => (
                    <button
                      key={screen.id}
                      type="button"
                      onClick={() => setActiveScreenId(screen.id)}
                      className={clsx(
                        'rounded-full border px-3 py-2 text-[11px] uppercase tracking-[0.24em] transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]',
                        screen.id === activeScreenId
                          ? 'border-[#f49748]/45 bg-[#f49748]/12 text-[#fff3e1]'
                          : 'border-white/10 bg-white/[0.02] text-[#b7aa9d] hover:border-white/20 hover:text-[#fbf6ef]'
                      )}
                    >
                      {screen.label}
                    </button>
                  ))}
                </div>

                <div className="mt-6 rounded-[24px] border border-[#5bc0be]/20 bg-[#5bc0be]/8 p-4">
                  <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                    <div className="max-w-2xl">
                      <p className="text-[11px] uppercase tracking-[0.28em] text-[#a8f2ec]">Generated route contract</p>
                      <h3 className="mt-2 text-xl font-semibold text-[#f4fffd]">{activeRouteContract.routeLabel}</h3>
                      <p className="mt-3 text-sm leading-6 text-[#dffaf7]">{activeRouteContract.rationale}</p>
                    </div>
                    <div className="rounded-full border border-[#5bc0be]/35 bg-[#07181b]/50 px-3 py-2 text-[11px] uppercase tracking-[0.24em] text-[#b8fff8]">
                      {activeRouteContract.enforcementMode}
                    </div>
                  </div>

                  <div className="mt-4 flex flex-wrap gap-2">
                    {activeRouteContract.allowedMethods.length > 0 ? (
                      activeRouteContract.allowedMethods.map((method) => (
                        <span
                          key={method}
                          className="rounded-full border border-[#5bc0be]/30 bg-[#0b2225] px-3 py-2 text-[11px] uppercase tracking-[0.18em] text-[#d9fffb]"
                        >
                          {method}
                        </span>
                      ))
                    ) : (
                      <span className="rounded-full border border-[#f49748]/35 bg-[#2a170c] px-3 py-2 text-[11px] uppercase tracking-[0.18em] text-[#ffd8b3]">
                        no direct-read rpc methods allowed
                      </span>
                    )}
                  </div>
                </div>

                <div className="mt-6 grid gap-4 md:grid-cols-3">
                  {activeScreen.cards.map((card) => (
                    <div
                      key={card.label}
                      className={clsx(
                        'rounded-[24px] border p-4 transition duration-500',
                        card.tone === 'alert'
                          ? 'border-[#6e2e37] bg-[#281218]'
                          : card.tone === 'warm'
                            ? 'border-[#6c4c1d] bg-[#24170d]'
                            : card.tone === 'cool'
                              ? 'border-[#2d575a] bg-[#0f1f25]'
                              : 'border-white/8 bg-white/[0.03]'
                      )}
                    >
                      <p className="text-[11px] uppercase tracking-[0.24em] text-[#9f9487]">{card.label}</p>
                      <p className="mt-3 text-2xl font-semibold text-[#fbf6ef]">{card.value}</p>
                      <p className="mt-3 text-sm leading-6 text-[#c5b7a7]">{card.detail}</p>
                    </div>
                  ))}
                </div>

                <div className="mt-6 grid gap-4 lg:grid-cols-[1.1fr_0.9fr]">
                  <section className="rounded-[24px] border border-white/7 bg-[#0d0a12] p-4">
                    <p className="text-[11px] uppercase tracking-[0.24em] text-[#9f9487]">Module stack</p>
                    <div className="mt-4 space-y-3">
                      {activeScreen.modules.map((module) => (
                        <article key={module.name} className="rounded-[18px] border border-white/7 bg-white/[0.03] p-4">
                          <div className="flex items-start justify-between gap-3">
                            <p className="text-sm font-medium text-[#fbf6ef]">{module.name}</p>
                            <span className="text-[10px] uppercase tracking-[0.24em] text-[#9f9487]">{module.status}</span>
                          </div>
                          <p className="mt-2 text-sm leading-6 text-[#bcae9f]">{module.detail}</p>
                        </article>
                      ))}
                    </div>
                  </section>

                  <section className="rounded-[24px] border border-white/7 bg-[#0d0a12] p-4">
                    <p className="text-[11px] uppercase tracking-[0.24em] text-[#9f9487]">Journey ladder</p>
                    <div className="mt-4 space-y-3">
                      {activeRoute.journey.map((step, index) => (
                        <button
                          key={step.title}
                          type="button"
                          onClick={() => setActiveJourneyStep(index)}
                          className={clsx(
                            'w-full rounded-[18px] border px-4 py-4 text-left transition duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f49748]',
                            index === activeJourneyStep
                              ? 'border-[#f49748]/35 bg-[#f49748]/10'
                              : 'border-white/7 bg-white/[0.03] hover:border-white/18'
                          )}
                        >
                          <div className="flex items-center justify-between gap-3">
                            <p className="text-sm font-medium text-[#fbf6ef]">{step.title}</p>
                            <span className="text-[10px] uppercase tracking-[0.24em] text-[#9f9487]">{step.state}</span>
                          </div>
                          <p className="mt-2 text-sm leading-6 text-[#bcae9f]">{step.detail}</p>
                        </button>
                      ))}
                    </div>

                    <div className="mt-4 rounded-[18px] border border-[#5bc0be]/20 bg-[#5bc0be]/8 p-4">
                      <p className="text-[11px] uppercase tracking-[0.24em] text-[#a8f2ec]">Focused transition</p>
                      <p className="mt-2 text-sm leading-6 text-[#e5fffc]">{highlightedJourney.detail}</p>
                    </div>
                  </section>
                </div>
              </article>

              <div className="grid gap-6">
                <article className="rounded-[28px] border border-white/8 bg-[#15111b] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6">
                  <p className="text-[11px] uppercase tracking-[0.28em] text-[#9f9487]">Scenario overlay</p>
                  <h3 className="mt-2 text-2xl font-semibold text-[#fbf6ef]">{activeScenario.label}</h3>
                  <p className="mt-3 text-sm leading-7 text-[#c5b7a7]">
                    This local state changes copy emphasis, highlighted modules, and the journey focus without touching any network call.
                  </p>
                  <div className="mt-5 rounded-[24px] border border-white/7 bg-white/[0.03] p-4">
                    <p className="text-[11px] uppercase tracking-[0.24em] text-[#9f9487]">Current state</p>
                    <p className="mt-2 text-lg font-medium text-[#fbf6ef]">{activeScenario.state}</p>
                    <p className="mt-3 text-sm leading-6 text-[#bcae9f]">{activeScenario.description}</p>
                  </div>
                </article>

                <article className="rounded-[28px] border border-white/8 bg-[#15111b] p-5 shadow-[0_18px_50px_rgba(0,0,0,0.32)] lg:p-6">
                  <p className="text-[11px] uppercase tracking-[0.28em] text-[#9f9487]">Isolation contract</p>
                  <h3 className="mt-2 text-2xl font-semibold text-[#fbf6ef]">Hard boundaries for this shell</h3>
                  <div className="mt-5 space-y-3">
                    {shellAlerts.map((alert) => (
                      <div key={alert} className="rounded-[22px] border border-white/7 bg-white/[0.03] p-4 text-sm leading-6 text-[#c5b7a7]">
                        {alert}
                      </div>
                    ))}
                  </div>

                  <div className="mt-6 space-y-3">
                    {shellFlowCards.map((card) => (
                      <div key={card.title} className="rounded-[18px] border border-white/7 bg-black/20 p-4">
                        <div className="flex items-center justify-between gap-3">
                          <p className="text-sm font-medium text-[#fff5ec]">{card.title}</p>
                          <span className="text-[10px] uppercase tracking-[0.24em] text-[#9f9487]">{card.state}</span>
                        </div>
                        <p className="mt-2 text-sm leading-6 text-[#bcae9f]">{card.description}</p>
                      </div>
                    ))}
                  </div>

                  <div className="mt-6 rounded-[24px] border border-[#5bc0be]/20 bg-[#5bc0be]/8 p-4">
                    <p className="text-[11px] uppercase tracking-[0.28em] text-[#a8f2ec]">How to open</p>
                    <p className="mt-3 text-sm leading-6 text-[#e5fffc]">
                      Run the Vite app and open the shell with <span className="font-semibold text-[#fff7ef]">?surface=shell</span>. The default route remains untouched.
                    </p>
                  </div>
                </article>
              </div>
            </section>
          </div>
        </section>
      </div>
    </main>
  );
}
