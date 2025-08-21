# Oppla AI IDE Release Notes - AI Profile System Launch
Date: 2025-08-17

We’ve introduced a new AI Profile System that tailors Oppla AI IDE to your current task. Choose from Architect, DevOps, or Marketing Specialist profiles to align the AI assistant with system design, infrastructure, or content marketing needs. This release includes new prompts, scoped tooling, and UI improvements to streamline your workflow and improve ROI.

Highlights
- 3 specialized AI profiles with distinct expertise and custom prompts
- Dynamic profile-based prompt loading and safe tooling permissions
- UI updates with descriptive profile information
- Backward compatibility with existing profiles (Write, Ask, Minimal)
- Profiles inherit project rules automatically

What’s new
1) Architect Profile
- Focus: system design, architecture patterns, scalability
- Expertise: SOLID principles, microservices, API design, cloud architecture
- Tools: full code access except terminal (for safety)

2) DevOps Profile
- Focus: infrastructure automation, CI/CD, cloud platforms
- Expertise: Terraform, Kubernetes, Docker, monitoring, SRE practices
- Tools: full terminal access for infrastructure management

3) Branding Specialist Profile
- Focus: content creation, SEO, brand strategy
- Expertise: digital marketing, analytics, customer engagement
- Tools: content editing, web search, file organization (no code execution)

Technical implementation highlights
- Extended Profile System: Added prompt_template and role_description fields to support per-profile prompts
- Custom Prompt Templates: Each profile ships with its own Handlebars (.hbs) template containing role-specific instructions
- Dynamic Prompt Loading: The system loads and applies the correct template when a profile is selected
- Tool Permissions: Profiles have tailored tool access aligned with their responsibilities
- UI Integration: Agent panel profile selector now includes descriptive text for all profiles

Key features and benefits
- Inheritance of project rules: Profiles automatically apply project .rules files and user-defined rules
- Backward compatibility: Existing profiles (Write, Ask, Minimal) work as before
- Consistent safety and formatting: All profiles adhere to the same code quality and safety guidelines
- Simple switching: Profiles can be chosen directly from the Agent panel dropdown to match the task at hand

How to use
- Open the Agent panel in Oppla AI IDE
- Open the profile dropdown and select Architect, DevOps, or Branding Specialist
- Review the descriptive text to understand the profile’s focus and capabilities
- Start working; the AI will load the appropriate prompt template and tool permissions automatically
- For project-specific prompts, ensure your project’s rules are defined; profiles will inherit them

Migration and compatibility notes
- Existing workflows remain intact; no breaking changes to current users
- Profiles can be mixed within a project; rules are applied per profile instance
- If you have custom prompts or prompts that rely on terminal access, review the new per-profile templates to align with capabilities

Impact and value
- Increased productivity through task-aligned AI assistance
- Reduced context switching by letting the AI handle domain-specific reasoning and tooling
- Clearer expectations and safer interactions due to explicit tool permissions per profile
- Enhanced onboarding for new team members who switch between architecture, operations, and content work

 rollout and availability
- Released to production with a targeted rollout
- Fully available to all users; no additional actions required to enable

Known limitations and considerations
- Architect profile: full code access only; terminal access is restricted for safety
- DevOps profile: includes full terminal access for infrastructure management
- Branding Specialist profile: restricted to content editing, web search, and file organization; no direct code execution
- As with any new AI prompt system, occasional profile overlap can occur; switching profiles mid-session may reset some context to the new profile’s perspective

Future improvements (tentative)
- More profiles and industry-specific prompts
- User-facing customization for prompt templates within supported boundaries
- Deeper analytics on profile adoption and task-time reductions
- Expanded pricing tiers to reflect added value in professional profiles

Measurable outcomes and ROI (suggested)
- Adoption rate of each profile by role
- Time-to-first-deliverable reductions per profile
- User satisfaction scores and feedback on profile guidance
- Increase in output quality and consistency for architecture diagrams, infra configurations, and marketing content

Call to action
- Try the new AI profiles today in your Agent panel and experience task-aligned AI assistance.
- Share your feedback to help us refine prompts and expand capabilities.
- If you’re evaluating better collaboration between product teams and marketing, consider discussing how these profiles can accelerate delivery—and reach out about exploring premium features or pricing options that unlock broader capabilities.

Marketing-focused tagline
- Choose the AI profile that fits your current mission—design scalable architectures, automate resilient infrastructure, or craft compelling marketing content—with built-in safety and project-aware guidelines.

Notes for product and growth teams
- Highlight the three new profiles in onboarding checklists and product tours
- Create quick-start templates and example prompts for each profile to demonstrate value
- Prepare short-form customer-facing materials that illustrate time-to-value metrics and ROI from using specialized profiles
