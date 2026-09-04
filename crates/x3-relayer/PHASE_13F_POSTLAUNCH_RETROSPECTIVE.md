# Phase 13f Post-Launch Retrospective Framework

**Purpose:** Systematic review of launch execution, learning capture, and continuous improvement  
**Timing:** Conduct within 5 days of launch completion (T+7d to T+12d)  
**Audience:** Full launch team, leadership, stakeholders  
**Duration:** 2-3 hour workshop session

---

## Pre-Retrospective Preparation (T+3d to T+7d)

### Data Collection

Assign one team member to gather:

**Metrics & Timeline:**
- Complete uptime log (T-0h to T+7d)
- Performance metrics (TPS, latency, resource usage)
- Incident timeline (all events with timestamps)
- Validator onboarding statistics
- Bridge activity metrics

**Incident Documentation:**
- All incidents from logs
- Root cause analyses
- Resolution steps taken
- Recovery time per incident
- Escalations and who was contacted

**Communication Log:**
- All status updates published
- Stakeholder feedback received
- Partner questions and answers
- Community feedback/sentiment
- Media mentions or coverage

**Team Feedback:**
- Send survey to all team members (see template below)
- Capture on-call shift notes
- Document any ad-hoc discussions
- Collect photos/screenshots of war room

**Process Documentation:**
- What procedures were followed
- What was skipped or modified
- Workarounds implemented
- Tools/systems that helped most
- Tools/systems that created friction

### Pre-Retrospective Survey (Send T+2d)

**To:** All launch team members  
**Due:** T+5d  
**Format:** Confidential survey (5 minutes)

```
PHASE 13f POST-LAUNCH SURVEY

Rate 1 (Strongly Disagree) to 5 (Strongly Agree):

1. The documentation was clear and easy to follow.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

2. The procedures prevented problems from occurring.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

3. When issues occurred, we had the right playbooks.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

4. The team communication was effective.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

5. I felt prepared for my role.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

6. The incident response process worked smoothly.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

7. Tools and monitoring provided needed visibility.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

8. I would change nothing about the launch execution.
   ☐ 1  ☐ 2  ☐ 3  ☐ 4  ☐ 5

Open-Ended Questions:

What went BEST during the launch? Why?

What was MOST DIFFICULT? Why?

What would you change for the next major operation?

What did the team do WELL that we should keep doing?

Any other feedback?

- Anonymous responses welcome
- Honest feedback appreciated
- No wrong answers
```

---

## Retrospective Workshop Agenda (T+7d)

**Duration:** 2-3 hours  
**Facilitator:** [Name] — ensure neutral, non-blaming tone  
**Attendees:** Entire launch team + key stakeholders  
**Format:** In-person or Zoom with recording

### Section 1: Opening & Context (15 minutes)

**Goal:** Align everyone on why we're doing this and what's safe to discuss.

**Agenda:**
1. Welcome everyone
2. Briefly recap the launch timeline (T-48h to T+7d)
3. Remind everyone: This is a learning session, not blame
4. All feedback is confidential — we focus on systems, not people
5. Outcomes will be documented and acted upon

**Key Message:** "We had a successful launch. This retrospective helps us learn and improve for the next operation."

---

### Section 2: Facts & Timeline (20 minutes)

**Goal:** Establish shared understanding of what actually happened.

**Facilitator:** [Data Lead]

**Present:**
- Timeline of all key events (T-0h to T+7d)
- Major metrics (uptime, TPS, latency, etc.)
- All incidents with detection time and recovery time
- Team shift coverage and on-call effectiveness
- Major decisions and decision points

**Discussion Questions:**
1. Does this timeline match everyone's experience?
2. What's missing or incorrect?
3. What was the most stressful moment?
4. When did we feel most confident?

**Document:** Agreed-upon timeline

---

### Section 3: What Went Well (20 minutes)

**Goal:** Celebrate wins and identify what to preserve.

**Format:** Structured brainstorm (one comment per person, no discussion yet)

**Prompts:**
- What documentation was most helpful?
- What procedure prevented problems?
- Who stepped up and helped when needed?
- What tool or system worked really well?
- What did the team do together that was great?

**Capture:** List all responses on whiteboard/document

**Synthesis:** Group into categories:
- Documentation/Procedures (what was clear)
- Tools/Systems (what enabled us)
- Team/Culture (what we did well)
- Communication (how we coordinated)

**Action:** Identify 3-5 things to "Keep Doing"

---

### Section 4: What Was Difficult (20 minutes)

**Goal:** Identify improvement areas without blame.

**Format:** Problem-focused brainstorm (not blame-focused)

**Prompts:**
- What documentation was confusing or missing?
- Where did procedures break down?
- What tools slowed us down?
- Where was communication unclear?
- What would you change?

**Capture:** All responses on whiteboard/document

**Important Note for Facilitator:**
- If someone says "John made a mistake," reframe: "The procedure didn't account for X situation. How do we prevent that next time?"
- If someone says "Documentation was unclear," ask: "Which specific section? What would make it clearer?"

**Synthesis:** Group into categories:
- Process gaps (procedures we need)
- Documentation gaps (clarity issues)
- Tool limitations (what we need)
- Communication issues (what wasn't clear)
- Capability gaps (skills we need)

**Action:** Identify 3-5 things to "Improve"

---

### Section 5: Incident Review (15 minutes per incident)

**Goal:** Understand each incident fully and extract learnings.

**For Each Incident (if any):**

**1. Incident Timeline**
- When detected: [Time]
- Root cause: [Analysis]
- Time to resolve: [Duration]
- Team members involved: [Names]

**2. Root Cause Analysis**
- What was the immediate cause?
- What was the underlying cause?
- Could we have prevented this?
- Could we have detected it sooner?

**3. Response Execution**
- Did we have a playbook?
- Did the playbook work?
- What ad-hoc troubleshooting occurred?
- How did communication work?

**4. Recovery & Verification**
- How did we confirm the fix?
- Did we check for side effects?
- When was it safe to declare "resolved"?

**5. Learnings**
- What should we change in procedures?
- What should we change in monitoring?
- What should we change in documentation?
- What training is needed?

**6. Action Items**
- [ ] Document [lesson] in [procedure]
- [ ] Update [monitoring] to catch this sooner
- [ ] Train team on [topic]

---

### Section 6: Team Performance & Wellness (15 minutes)

**Goal:** Acknowledge effort, identify burnout risks, and celebrate team.

**Discussion Points:**
- How did everyone manage the sleep/rest during on-call periods?
- What was the emotional experience (stress, confidence, achievement)?
- Were shifts appropriately sized?
- Did team members feel supported?
- Any burnout concerns we should address?

**Recognition:**
- Call out specific examples of excellence
- Thank people by name for contributions
- Acknowledge difficult decisions and trades

**Future Considerations:**
- Should we have different shift patterns next time?
- What support/tools would help team wellness?
- How do we maintain this energy across a longer operation?

---

### Section 7: Documentation & Procedures Assessment (15 minutes)

**Goal:** Evaluate documentation quality and identify updates needed.

**For Each Major Document:**

**PHASE_13F_MAINNET_LAUNCH_RUNBOOK:**
- Was it used during launch?
- Was it accurate?
- What sections were helpful?
- What sections were confusing or missing?
- Grade: A / B / C / D

**MAINNET_INCIDENT_RESPONSE:**
- Did we use incident playbooks?
- Were they helpful?
- What scenarios need documentation?
- What scenarios were well-covered?

**RPC_FAILOVER_PROCEDURES:**
- Was failover smooth?
- Did we use manual procedures?
- What was missing?

**VALIDATOR_OPERATIONS:**
- Validator onboarding go smoothly?
- Any issues with procedures?

**Other Documents:**
- GPU_VALIDATOR_TROUBLESHOOTING
- MAINNET_PERFORMANCE_BASELINE
- Daily status template
- Communication templates

**Actions:**
- [ ] Update [doc] with [correction]
- [ ] Add new section to [doc]
- [ ] Add examples/screenshots to [doc]

---

### Section 8: Recommendations for Future Launches (20 minutes)

**Goal:** Capture systematic improvements for next operation.

**Discussion Categories:**

**Preparation Phase (T-96h to T-48h):**
- What should we do differently?
- What training is needed?
- What infrastructure improvements?
- What documentation improvements?

**Execution Phase (T-48h to T+7d):**
- What worked and should we repeat?
- What changes to procedures?
- What communication improvements?
- What monitoring/alerting improvements?

**Post-Launch Phase (T+7d to T+30d):**
- What should we track long-term?
- When do we declare "stable"?
- When do we shift to business-as-usual?

**Technology & Tools:**
- What new tools would help?
- What tools should we retire?
- What monitoring improvements?

**Team & Skills:**
- What training is most valuable?
- What role improvements?
- What on-call model improvements?

**Organization & Process:**
- What communication improvements?
- What decision-making improvements?
- What escalation improvements?

**Create Action Items:**
- [ ] [Recommendation] owner: [Name] target: [Date]

---

### Section 9: Closing & Commitments (10 minutes)

**Goal:** Ensure accountability and close the retrospective.

**Actions:**

1. **List all action items from retrospective** (see below)

2. **Assign owners and target dates:**
   - [ ] [Action] — Owner: [Name] — Target: [Date]

3. **Thank the team:**
   - "This launch was successful because of your professionalism, preparation, and dedication."
   - Specific thanks to [people] for [contributions]

4. **Celebrate:**
   - Take a photo of the team
   - Share results with broader organization
   - Acknowledge in company newsletter/meeting

5. **Next Steps:**
   - Retrospective summary will be documented
   - Action items will be tracked
   - Progress updates at [date]
   - Next launch will benefit from these learnings

---

## Post-Retrospective Documentation (T+7d to T+14d)

### Create Retrospective Report

**Author:** [Assigned person]  
**Due:** T+10d  
**Format:** Markdown document

**Structure:**
1. Executive Summary (1 page)
   - Launch was successful (metrics)
   - Key achievements
   - Key improvements needed

2. Detailed Findings (per section above)
   - What went well
   - What was difficult
   - Incident analyses
   - Team feedback summary

3. Action Items & Owners (with tracking)
   - [ ] [Action] — Owner: [Name] — Due: [Date] — Priority: [1-5]

4. Appendix
   - Raw survey responses
   - Timeline of events
   - Metrics summary
   - Photos/artifacts

### Action Item Tracking

**Create shared tracker (spreadsheet or issue board):**

| Action Item | Category | Owner | Due Date | Priority | Status |
|-------------|----------|-------|----------|----------|--------|
| Update LAUNCH_RUNBOOK with [issue] | Docs | [Name] | [Date] | 1 | ⏳ In Progress |
| Add monitoring for [metric] | Monitoring | [Name] | [Date] | 2 | ⏳ In Progress |
| Train team on [topic] | Training | [Name] | [Date] | 2 | 📋 Scheduled |

**Review & Update:** Weekly at [day] [time] until all items complete

### Communicate Results

**Within 24 hours of retrospective:**

✉️ **Email to all attendees:**
- Thank you for participation
- Summary of key findings
- Timeline for addressing action items
- Link to full report

📋 **Present to leadership:**
- What went well
- What we're improving
- Timeline for improvements

🔗 **Archive documentation:**
- Retrospective report → `/docs/retrospectives/`
- Action items → Tracking system
- Videos/recordings → Team wiki

---

## Retrospective Template — What Success Looks Like

### Launch Performance ✅

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Uptime | 99.5%+ | [X%] | ✅ |
| Throughput | 100+ TPS | [X] TPS | ✅ |
| Latency | < 30s | [X]s | ✅ |
| MTTR (incidents) | < 30 min | [X] min | ✅ |
| Team wellness | No burnout | [Result] | ✅ |

### Learning Capture ✅

✅ All incidents documented with root causes  
✅ All procedures validated (or updated)  
✅ All team feedback collected and categorized  
✅ All action items assigned with owners and dates  
✅ All improvements scheduled into next phases

### Continuous Improvement ✅

✅ [N] action items for procedure improvements  
✅ [N] action items for monitoring enhancements  
✅ [N] action items for training needs  
✅ [N] action items for tooling/automation

---

## Example: Strong Retrospective Results

**Sample Findings:**

**What Went Well:**
- Documentation was 90% accurate
- Incident playbooks prevented 3 escalations
- Team communication via Slack was clear
- Monitoring alerting caught all issues
- Validator onboarding was smooth

**What Needs Improvement:**
- T+1h to T+4h confusion about shift handoff (update procedures)
- RPC latency not tracked in monitoring (add metric)
- Communication templates were late (pre-create next time)
- Missing GPU troubleshooting scenario (add to playbooks)
- New team members needed more prep (add training)

**Action Items (Sample):**
- [ ] Update LAUNCH_RUNBOOK T+1h section — Owner: Alice — Due: T+14d — P1
- [ ] Add RPC latency tracking to Prometheus — Owner: Bob — Due: T+21d — P2
- [ ] Pre-write all comms templates — Owner: Carol — Due: T+30d — P2
- [ ] Add GPU scenario to INCIDENT_RESPONSE — Owner: Dave — Due: T+21d — P2
- [ ] Create "New Team Member Onboarding" doc — Owner: Eve — Due: T+30d — P3

---

## Facilitation Tips for Retrospective Leaders

### Create Psychological Safety

- Explicitly state: "No blame, no judgment"
- Go first with vulnerability ("I made a mistake because...")
- Acknowledge difficult situations neutrally
- Thank people for honest feedback

### Keep Focus

- Stay on "What can we learn and improve?" not "Who made mistakes?"
- Interrupt blame → redirect to systems
- Keep moving (don't litigate every detail)
- Capture tangents for later follow-up

### Ensure Participation

- Use round-robin sharing (everyone contributes)
- Provide quiet reflection time before discussion
- Write-first (write on board), then discuss
- Ask clarifying questions, don't interrogate

### Balance Positivity & Criticism

- Start with "What Went Well" before "What Was Difficult"
- Celebrate openly, improve thoughtfully
- Don't let criticism overshadow success
- End with action items (forward-looking)

### Handle Conflict

**If someone criticizes another person:**
- "Let's focus on the system. Given X situation, how do we design a process to prevent that?"

**If someone's defensive:**
- "I hear you. That's valuable context. What would make the procedure clearer for next time?"

**If disagreement occurs:**
- "Good points on both sides. Let's note both for the action items."

---

## Post-Retrospective Success

✅ **Retrospective is successful if:**
- Everyone felt heard
- Action items are concrete and owned
- Team feels energized, not blamed
- Improvements are scheduled
- Leadership understands learnings
- Team is ready to do it again

❌ **Red flags (address immediately):**
- Someone felt blamed or defensive
- Action items are vague ("improve X")
- Leadership doesn't understand why changes matter
- Changes aren't scheduled/resourced
- Team suggests we "do exactly the same thing"

---

## Appendix: Follow-Up Meetings

### T+14d: Action Items Status Update
- [ ] All action items assigned?
- [ ] All owners confirmed?
- [ ] All target dates realistic?

### T+21d: 2-Week Progress Check
- [ ] [N]% of action items in progress
- [ ] Any blockers?
- [ ] Any urgent items that need acceleration?

### T+30d: Action Items Review
- [ ] How many completed?
- [ ] Which ones are taking longer? Why?
- [ ] Any new action items discovered?

### T+60d: Retrospective Lessons Validation
- [ ] Have we actually made the improvements?
- [ ] Are we using the improved procedures?
- [ ] Is the team following new guidelines?

---

**Document Version:** 1.0  
**Last Updated:** April 21, 2026  
**Status:** Ready for Post-Launch Use

