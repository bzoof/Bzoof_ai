---
name: backend-development-facilitation
description: "Guide backend implementation planning, API design, data architecture, and system integration for a feature or service."
argument-hint: What backend feature, service, or technical area should this skill support?
disable-model-invocation: true
---
Related skill: `agent-customization`. Load and follow **skills.md** for template and principles.

This skill helps a backend developer or engineering lead translate requirements into technical tasks, design APIs and data models, and prepare implementation-ready work.

## When to use
- defining backend architecture, API contracts, and data workflows
- planning implementation for services, integrations, or data processing
- aligning backend work with frontend, DevOps, and security requirements

## Workflow
1. Understand requirements and scope
   - review feature requirements, acceptance criteria, and user flows
   - identify backend responsibilities, data needs, and system boundaries
   - capture dependencies on frontend, third-party services, and infrastructure
2. Design APIs and data models
   - define REST/GraphQL/API contracts, request/response shapes, and error handling
   - model domain entities, data storage, and validation rules
   - include security, performance, and scalability considerations
3. Plan implementation and integration
   - break work into backend tasks, stories, and implementation steps
   - select frameworks, libraries, database patterns, and middleware
   - define integration points, queueing, caching, and orchestration needs
4. Validate and review
   - review the design with frontend, DevOps, and security teams
   - identify potential bottlenecks, failure modes, and edge cases
   - update the plan with feedback and risk mitigations
5. Prepare delivery-ready artifacts
   - document API contracts, data schemas, and deployment requirements
   - create a task list, test plan, and acceptance criteria
   - ensure alignment with sprint scope and architecture decisions

## Completion criteria
- clear backend responsibilities and scope
- documented API contracts and data models
- implementation plan with prioritized tasks and dependencies
- identified security, performance, and reliability considerations
- alignment with frontend, DevOps, and architecture goals

## Questions to ask
- What are the expected inputs, outputs, and error conditions for the backend service?
- What data must be stored, retrieved, or transformed?
- Which external services or systems must the backend integrate with?
- What are the operational expectations for latency, throughput, and reliability?
- What security controls, authentication, or authorization requirements apply?
