export type Proof = {
  id: string
  status: string
  reference: string
  description?: string
  recordedAt?: string
  reviewer?: string
}

export type WorkPackage = {
  id: string
  title: string
  description: string
  linkedFindings: string[]
  requested: number
  pledged: number
  received: number
  allocated: number
  spent: number
  status: string
  evidenceStatus: string
  proofIds: string[]
}

export type PortalData = {
  generatedAt: string
  source: {
    snapshotPath: string
    sourceFingerprint: string
    sourceCommit: string | null
    readinessGeneratedAt: string | null
    evidenceAssets: string[]
  }
  funding: { currency: string; asOf: string; requested: number; pledged: number; received: number; allocated: number; spent: number }
  workPackages: WorkPackage[]
  proofLedger: Proof[]
  completion: {
    readinessScore: number
    uncappedScore: number
    taskCount: number
    completedTasks: number
    taskProgressPercent: number
    openFindings: Record<string, number>
    releaseDecision: string
    checks: { id: string; status: string; reason: string; exitCode: number | null; evidenceId: string | null; log: string | null }[]
    baselineEligible: boolean
  }
}
