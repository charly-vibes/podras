## MODIFIED Requirements

### Requirement: Bounded sections with declared truncation

The agent envelope SHALL apply bounded defaults: 1 focus, 15 in flight,
5 recent, 5 blocked, 5 next, and 5 parked items, while the warnings list SHALL
remain unbounded. Whenever any bound removes items, the envelope SHALL set
`truncation.applied = true` and SHALL record the per-section removed count in
`truncation.omitted_counts`. Silent truncation SHALL NOT occur.

#### Scenario: bounds_declared

- **WHEN** a fixture repository has more items per section than the declared bounds
- **THEN** the envelope section counts equal the declared bounds (1 focus, 15 in flight, 5 recent, 5 blocked, 5 next, 5 parked) and the warnings section remains unbounded
- **VERIFIES** [[brief.v1.bounds_declared]]

#### Scenario: truncation_flagged

- **WHEN** a fixture repository produces more than five recent events
- **THEN** `truncation.applied` is true and `omitted_counts.recent` equals the total recent events minus five
- **VERIFIES** [[brief.v1.truncation_flagged]]

#### Scenario: in_flight_truncated

- **WHEN** a fixture repository has more than fifteen in-flight items
- **THEN** the envelope keeps the first fifteen items of the declared priority-then-id order, `truncation.applied` is true, and `omitted_counts.in_flight` equals the total in-flight items minus fifteen
- **VERIFIES** [[brief.v1.in_flight_truncated]]
