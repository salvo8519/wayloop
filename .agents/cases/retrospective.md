# Behavioural case: retrospective

## Input
Milestone M0 is closed. It had twelve tasks: nine closed as completed, one closed as not planned, two moved to M1 during the milestone. Journal entries exist for five of the issues; one of the moved tasks waited three weeks on `needs-decision`.

## Expected
An inventory that lists the sources read and states that seven issues have no journal entry. A table with nine shipped, one dropped and two moved tasks, each citing its issue and pull request. The slip of the task waiting on a decision is attributed to the open decision, with the issue as evidence; the other moved task reads "cause not recorded" if no evidence explains it. Cost signals per issue, with token cost reported as not available. At most five proposals, each a roadmap change or an issue for a process file; none is applied, and the confirmed ones are handed to `roadmap-update`.
