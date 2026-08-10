> 📋 **[SOURCE NOTE - MSc FINAL REPORT, EDITABLE SOURCE]**
> Student: Lukasz Robak B00459151 | Supervisor: Jacob Koenig
> Deadline: 14 August 2026 | Limit: 18,000 words (~60 pages)
> Build to docx: PYTHONIOENCODING=utf-8 /c/Users/lukii/miniconda3/python build_docx.py (add "final" for the clean submission build). FRONT MATTER is added by the student, NOT by this script: the UWS dissertation cover sheet (from !template documents/DissertationFrontSheet.doc) and the Declaration of originality are prepended manually to the compiled docx, each on its own page before the Contents.
>
> NOTE CONVENTION: everything that does NOT go into the compiled docx is written as an emoji blockquote like this one. Every blockquote opened with "> <emoji>" is stripped at compile, including its "> " continuation lines. A note must be followed by a BLANK line before any report content.
> - 🟢 [DRAFTED] - section status | 🟠 [EXPANSION NOTE] - content to add later | 📝 [WORKING NOTE] - working discussion | 🖼️ [FIGURE NOTE] - figure provenance | 📋 [SOURCE NOTE] - this header
> - 🧭 [TABLE OF CONTENTS] / 🗂️ [TABLE OF FIGURES] / 📑 [LIST OF TABLES] - compile inserts the auto-updatable Word fields at these markers
>
> TBC is the only allowed placeholder ("TBC" or "TBC (clarification)"). It passes into the (interim) docx and blocks a (final) build.
>
> FIGURE RULES: all figures 15cm wide (applied by build_docx.py), centered; caption "Figure <nr> - <short description>" centered below, one line; no self-attribution. Every generated PNG is visually checked before embedding.
> TABLE RULES: caption "Table <nr> - <short description>" on its own line ABOVE the table; build_docx.py styles it "Table Caption" for the auto List of Tables.
>
> WRITING RULES: no semicolon joining clauses, no em-dashes (use " - "), no "leverage", UK spelling, vendor names generic in prose. IMPERSONAL VOICE - no first person, "the author" only where unavoidable. PLAIN ENGLISH (rule added 11-13/07): no insider jargon anywhere - no "harness" (say test program), "battery" (test tasks), "probe" (check question), "ablation" (mechanism-removal run), "rubric" (scoring checklist), "golden" (master); simple, direct sentences in the author's own plain voice, common words over fancy ones. Scripted experiment prompts quoted verbatim keep their original wording.
> SCORING-STATUS WORDING (author's ruling 22/07, replaces the 13/07 wording): the report states that the author made the consistency verdicts - the author went through the completed answer sheets item by item against the archived transcripts and made every judgement, recording an evidence note per item. The report never describes any automated verdict pipeline and never says "blinded". Mechanical text checks remain correct wording ONLY for the binary task-quality checklists, which are unambiguous yes/no items.
> INLINE BOLD RULE: this md source MAY use **bold** for working emphasis; build_docx.py clears run-level bold from body prose at compile - use italics for emphasis that must survive.

# Designing and Evaluating a Self-Learning, Infinite-Memory Protocols MCP Agent for Using Both Persistent Knowledge and Expected Behaviour Across Independent LLM Chat Sessions

**MSc Artificial Intelligence - Final Project Report**
Lukasz Robak, B00459151
Supervisor: Jacob Koenig
School of Computing, Engineering and Physical Sciences, University of the West of Scotland

# Abstract

Between one chat session and the next, a large language model keeps nothing: its only working knowledge is the text currently inside its context window, and that window empties when the session ends. This project designed, built and evaluated a layer that adds what the model lacks: a self-learning routine that writes verified lessons into a structured knowledge base, and a memory independent of the window, read on demand through a three-level retrieval path with a grounding check. The layer attaches to an unmodified client as an MCP agent and detaches without trace, which makes a controlled experiment possible: the identical model, prompts and scenarios ran with the layer attached and absent, against a deliberately dry baseline - no tools, no file access, no provider-side memory, verified before every execution. The programme ran on two scenarios in three configurations: a small local engine with a micro-reader, the same engine reading for itself, and a large hosted engine. The dry model lost every planted fact, decision and rule at every size, and said so rather than guessing. With the layer attached the same model kept two thirds to nine tenths of the planted knowledge, rising with the engine and its reader arrangement, at a cost of roughly seven to forty-five times the dry baseline's input tokens. The losses concentrate at write time: retrieval stayed trustworthy in every configuration, and the quality of the notes the engine writes for itself is what sets the layer's ceiling.

> 🧭 **[TABLE OF CONTENTS]** The compile replaces this note with a "Contents" heading and an auto-updatable Word TOC field (root chapters only). Root chapters match the agreed marking scheme one-to-one (decision 15/07, see RESTRUCTURE-PLAN-2026-07-15.md). The Abstract sits before this Contents page (added 01/08) and, being a root heading, appears in the refreshed TOC like References and Appendices do. Source preview:
>
> 1. Introduction (5%)
> 2. Context (20%)
> 3. Research Design (15%)
> 4. Implementation (30%)
> 5. Analysis and Evaluation (10%)
> 6. Conclusions and Recommendations (10%)
> 7. Critical Self-Evaluation (10%)
>
> References | Appendices

> 🗂️ **[TABLE OF FIGURES]** Compile inserts the auto Table of Figures here, fed by the caption-styled captions. Source preview:
>
> - Figure 2.1 - The context window as a sliding boundary over a growing project history
> - Figure 2.2 - Where memory can live: in the weights, in the window, outside the model
> - Figure 2.3 - The MCP client-server-tool relationship
> - Figure 3.1 - The experimental design: paired ON/OFF executions over three sessions
> - Figure 3.2 - The planted-item method: plant and update early, check late
> - Figure 4.1 - One engine, many specialised knowledge agents: the artefact's stack
> - Figure 4.2 - The three-tier knowledge architecture of an agent
> - Figure 4.3 - The three-level retrieval path with the grounding check
> - Figure 4.4 - The self-learning write-back loop
> - Figure 4.5 - The three execution configurations: engine and reader arrangement
> - Figure 4.6 - Cross-session consistency by scenario, condition and configuration
> - Figure 4.7 - Task quality by scenario, condition and configuration
> - Figure 4.8 - Chat input tokens per execution, ON against OFF, all three configurations

> 📑 **[LIST OF TABLES]** Compile inserts the auto List of Tables here, fed by the "Table Caption"-styled captions. Source preview:
>
> - Table 4.1 - The programme as executed in the three configurations (clean sets)
> - Table 4.2 - Cross-session consistency on planted-item check questions (final verdicts, all three configurations)
> - Table 4.3 - Task quality: share of checklist criteria met (mechanical checks on chat answers)
> - Table 4.4 - The layer's cost per execution (chat model, all three configurations)
> - Table 4.5 - W2 check questions honoured by judgement type (ON condition, per execution)
> - (Tables F.1 to F.6 - the scenario materials - live in Appendix F's external file `SCENARIO-MATERIALS.md`, outside this docx)

> 🖼️ **[FIGURE NOTE]** All figures are own diagrams, generated reproducibly by figures/make_figures.py (matplotlib, 300 dpi PNG). Figures 4.6-4.8 carry the results of all three Experiment B configurations (values from 03b. Experiment B/v01/report/RESULTS.md + ANALYSIS.md, 2026-07-15 basis, v02/report/RESULTS.md + ANALYSIS.md, 2026-07-16 basis, and v03/report/RESULTS.md + ANALYSIS.md, 2026-07-16 attempt-3 basis). Rebuild: PYTHONIOENCODING=utf-8 /c/Users/lukii/miniconda3/python figures/make_figures.py.


---

# 1. Introduction

> 🟢 **[DRAFTED]** Re-voiced and shortened 13/07 (plain-language pass, final scoring numbers). Restructured 15/07: root chapters now mirror the marking scheme, old Chapter 3 (aim and objectives) folded in here as Section 1.3.

## 1.1 The practical problem and the response

A large language model answers from whatever text sits inside its context window at that moment, and when the session closes it keeps nothing. For a one-off question this hardly matters. For a project that runs over weeks it matters a lot, because the same model works on the same codebase again and again, and every time it starts from zero. It re-reads conventions it was already told, asks again about decisions that were already made, and repeats mistakes that were corrected only a few sessions earlier. A larger window does not fix this. It only delays the moment the project outgrows it, and it raises the token cost of every turn on the way.

This is the practical problem the project is built around. An agent was designed, built and evaluated that gives an ordinary model two things it does not have on its own. The first is a self-learning routine: after a task is finished, a verified lesson is written back into a structured knowledge base, so what was learned in one session is available in the next. The second is a memory that does not depend on the context window: knowledge is fetched on demand through three retrieval levels - a fast local search over the project files, the recorded guidelines supplied as fixed context, and a small second model used as a reading layer to keep the token cost low. The agent is implemented on the MCP (Model Context Protocol) standard as a set of specialised knowledge agents built on one engine, so the whole layer attaches to a standard model client from the outside and never changes the base model itself. That separation is the point of the design: the identical model can be run with the layer switched on and switched off, and the outcomes compared.

## 1.2 The question, and the headline of the answer

The question this project answers is narrow on purpose. Holding the base model, the scenario and the prompts fixed, does attaching this layer measurably improve task quality, consistency across sessions, rule-following and factual accuracy - and what does it cost in tokens? Nothing else is allowed to change, so any difference can be attributed to the layer. The layer is evaluated as one integrated whole: attached complete, or absent entirely.

The baseline is deliberately extreme: a completely dry model - no tools, no file access, no internet, no memory on the provider's side, verified before every execution - so that the only possible carrier of knowledge between sessions is the layer under test. The programme ran identically on two engine sizes from the same open-weight family - a small local engine and a large one served through the model publisher's cloud endpoint - and its headline can be given in three sentences. The dry model lost every fact, decision, rule and value judgement agreed in conversation, at both sizes, and said so honestly, guessing nothing. The identical model with the layer attached kept two thirds of everything planted on the code scenario and about half on the systems-analysis scenario at the small size, and roughly nine tenths on both scenarios at the large size - the layer's effect exists at both sizes and grows with the engine. The price is roughly seven to nine times the dry baseline's input tokens at the small size and thirty-five to forty-five times at the large, metered size, where a complete three-session working programme still costs about half a dollar.

## 1.3 Aim, research question and objectives

> 🟢 **[DRAFTED]** Verbatim from the approved specification (moved here from the old Chapter 3 in the 15/07 restructure). 22/07: pointer parentheses added after each objective, the separate chapter-mapping paragraph removed (author's rule: no text about how the report is constructed).

The aim of the project is to design, build and evaluate an MCP agent that adds a self-learning protocol and a context-window-independent memory to a standard large language model, and to establish, under controlled conditions, whether this added layer causes better and more consistent outcomes than an identically configured baseline, and to identify which mechanism is responsible.

The research question is as follows: holding the base language model, the workspace and the prompts constant, does enabling a self-learning and persistent-memory layer (an MCP agent with cross-session learning and smart multi-level knowledge retrieval technique) produce a measurable improvement in task correctness, cross-session consistency, consistent and repeatable guideline- and rule-based behaviour, and factual accuracy compared with the same model run without that layer, and at what cost in tokens?

The objectives of the project are:

1. Review the literature on the context-window limit, retrieval augmentation, agent memory architectures, self-improving agents and Recursive Language Models, and identify the gap in controlled artefact-level evaluation (Chapter 2).
2. Specify the agent architecture: the self-learning protocol (lesson capture and routing) and the three retrieval levels (local grep, full guideline context, external-model retrieval reader) (Sections 4.1 to 4.6).
3. Implement the agent as an MCP server attached to an unmodified base model, exposing a single switch that enables or disables the self-learning and memory layer (Sections 4.2, 4.7 and 4.8).
4. Build a repeatable evaluation scheme, ideally using also the long-prompt benchmarks for the fixed workspaces, run across several sessions with defined measures: correctness, cross-session consistency, follow-defined-rules consistency, factual accuracy, tokens consumption (Chapter 3).
5. Run the controlled comparison, toggling the layer on and off while the base model, workspace and prompts stay constant for recording outputs and traces (Sections 4.9 to 4.15).
6. Analyse the results and draw conclusions and future work recommendations (Chapters 5 and 6).

# 2. Context

> 🟢 **[DRAFTED]** Chain kept per the 30/06 and 06/07 requirements (GenAI -> LLM -> transformer -> window -> memory -> protocol -> MCP -> artefact). Shortened and re-voiced 13/07.

## 2.1 Generative AI and large language models

Generative artificial intelligence covers systems that produce new content - text, images, code - rather than classify existing content. This project concerns the large language model: a neural network trained on very large volumes of text to do one deceptively simple thing - given a sequence of text, predict what comes next. Everything a modern assistant appears to do is produced by repeatedly predicting the next small unit of text, called a token, and feeding the growing sequence back into itself. A token is roughly a short word or a fragment of a word, and it is the unit everything else in this report is measured in, including cost.

Two consequences matter here. First, the model's general knowledge lives in its parameters - billions of numeric weights fixed during training - and cannot be changed afterwards except by further training, which is far beyond an individual project and is in any case the wrong tool for keeping the small, changing facts of day-to-day work - for example, that a team picked a particular database last Tuesday. Second, everything the model knows about *your* situation has to be given to it as text, every time, in its input.

## 2.2 The transformer architecture

Almost every modern large language model is built on one architecture, the transformer, introduced by Vaswani et al. (2017). A rough sense of how it works explains both what these models are good at and the limit this project is concerned with.

Earlier language models read a sequence one item at a time, carrying a running summary forward, which made links between distant words hard to keep. The transformer replaced that with self-attention: the input is cut into tokens, each token becomes a vector of numbers, and every token looks at every other token at once, weighing how much each matters for interpreting it - so a pronoun can attend directly to the noun it refers to many words earlier. Stacking many attention layers builds a context-sensitive picture of the whole input, from which the next token is predicted.

Two properties carry through the report. Because every token attends to every other, the transformer handles long, structured material well. But for the same reason the input is not free: computation grows sharply with the number of tokens, so a longer input costs more on every call. The fixed budget of tokens the model can attend to in one call is the context window.

## 2.3 The context window

The context window is the maximum amount of text, measured in tokens, that the model can attend to in a single call - system instructions, the conversation so far, any files pasted in, and the answer being generated, all together. It is a hard boundary. Text beyond it simply does not exist for the model.

> 🖼️ **[FIGURE NOTE]** Figure 2.1: own work, make_figures.py (fig_2_1).

![](figures/figure-2-1.png)

*Figure 2.1 - The context window as a sliding boundary over a growing project history*

Three properties of the window shape the project. It is *finite*, and a long-lived project accumulates history faster than windows grow. It is *priced*, because commercial models charge per token read and written. And it is *transient*: when the session ends, the window's contents vanish, and nothing carries to the next session unless something outside the model carries it.

When a session grows past the boundary, tools either silently drop the oldest turns or summarise them into a shorter, lossy form. The trouble does not even wait for the hard boundary: output quality falls as the window fills (Wu et al., 2025). Enlarging the window is no answer - a bigger window still starts empty each session, costs more on every turn, and is used unevenly, with material buried in the middle attended to less reliably (Li et al., 2025).

## 2.4 Memory for a language model

"Memory" is used loosely in this field, so this section fixes the terms. A language model has exactly three places knowledge can live. *In the weights*, put there by training - general, fixed, expensive to change. *In the window* - perfectly accessible but transient and paid for on every turn. Or *outside the model* - in files, databases or notes - where it persists and costs nothing to keep, but where it is invisible until some mechanism selects the right piece and places it into the window at the right moment.

> 🖼️ **[FIGURE NOTE]** Figure 2.2: own work, make_figures.py (fig_2_2).

![](figures/figure-2-2.png)

*Figure 2.2 - Where memory can live: in the weights, in the window, outside the model*

External memory is therefore not one problem but three, and the split structures both the literature review and the artefact's design. The *writing* problem: deciding what is worth keeping from a session and recording it in an organised form. The *retrieval* problem: given a new task, finding the few stored items that matter for it. And the *grounding* problem: making sure what comes back from memory is actually supported by what was stored, because a memory that occasionally invents is worse than no memory at all.

Such memory is useful because real work repeats itself. A team's conventions, settled decisions and past corrections recur across sessions, and without external memory the assistant re-learns them from zero each time, which benchmark studies confirm at scale (Wu et al., 2025). The standard implementation is retrieval-augmented generation, where an index over external documents supplies relevant passages into the window at query time (Lewis et al., 2020). Beyond that sit architectures that page information in and out of external stores (Packer et al., 2023), consolidate past dialogues into compact records (Chhikara et al., 2025), or keep the full source outside the window and let the model inspect it piece by piece (Zhang, Kraska and Khattab, 2025). Sections 2.7 to 2.13 review these properly. All answer the retrieval problem in different ways, while the writing problem - what a working agent should *learn* from a finished task - is answered far less often.

Whichever architecture supplies the memory, the test of it is the same: memory counts only if it changes what the assistant does later. The instruments the field has settled on measure exactly that. They ask the model questions whose answers were established earlier in a long interaction - extracting stated facts, reasoning across sessions, handling updates where a later decision replaces an earlier one, and abstaining where nothing was ever established (Wu et al., 2025). Factual faithfulness is measured by breaking an answer into individual claims and checking each against a trusted source (Min et al., 2023). This study borrows the structure of these instruments directly.

## 2.5 From chat to tools: protocols and MCP

A language model by itself can only exchange text. For it to search files, run a program or write into a knowledge base, the application hosting it must offer those actions as *tools* - named operations the model may ask to invoke, with the host executing the call and returning the result into the window. Tool use is what turns a chat model into an agent: a model that pursues a task by choosing actions, observing results, and continuing.

Until recently every application wired tools to its model in its own way. The Model Context Protocol (MCP) is an open standard that fixes this. The host application runs an MCP *client*, and any number of MCP *servers* connect to it, each advertising the tools it offers. The host lists the tools to the model, the model requests a call, the server executes it and returns the result.

> 🖼️ **[FIGURE NOTE]** Figure 2.3: own work, make_figures.py (fig_2_3).

![](figures/figure-2-3.png)

*Figure 2.3 - The MCP client-server-tool relationship*

Two properties of MCP carry this project. A server is *external and detachable* - it attaches to an unmodified client and can be removed cleanly, without changing the model, the client or the workspace. And it is *host-independent*. The detachability is what makes the research design possible at all: the entire memory-and-learning layer can be present in one run and absent in the next, with everything else byte-for-byte identical. That clean experimental switch matters here, because the memory systems reviewed later in this chapter were not evaluated that way: their published comparisons set one memory method against another, not the same system against itself with the memory removed (Section 2.9).

## 2.6 What was created, in outline

The chain ends at a specific gap. Models are capable but windowed. External memory is the only durable store, and it splits into writing, retrieval and grounding. MCP makes it possible to package all three as a detachable layer on an unmodified client. The artefact - called extended-rlm in this report - is exactly that package: a layer of MCP servers, specialised knowledge agents built on one engine, adding a self-learning write-back routine and a three-level, window-independent retrieval path over tiered knowledge bases, with a grounding check on everything retrieval asserts. The name points back to Recursive Language Models (Zhang, Kraska and Khattab, 2025), whose central idea - keep the long context outside the window as an object to inspect, rather than text to squeeze in - the retrieval path follows at every level.

One design principle ties it together. If output quality falls as the prompt grows, and compressing history to fit is itself lossy, then the way to protect quality is to keep every prompt as small as it can be while still carrying everything the task needs - minimal but sufficient. So instead of pouring history into the window, the artefact reaches into a knowledge base of, in principle, unbounded size and returns only the slice that matters for the question in hand. Showing that this works is one of the things the report sets out to establish.

What the artefact is *for* is equally specific: it is the manipulated variable of a controlled experiment. The claim is not that a useful tool was built. The claim is causal - with everything else held constant, enabling this layer changes measurable outcomes on realistic multi-session work, by this much, at this token cost.

## 2.7 The literature: one line of argument

> 🟢 **[DRAFTED]** Shortened and re-voiced 13/07 (dropped three decorative citations). Line of argument unchanged: the mechanisms are well studied separately, but the causal question of what one integrated agent gains from its memory layer on real work has not been answered. Renumbered into Chapter 2 in the 15/07 restructure (old Chapter 4).

This review follows one line of argument rather than cataloguing the field. Each mechanism the evaluated agent combines - retrieval, agent memory, learning from experience - has its own mature literature, and the instruments for measuring memory are now good. What the literature does not contain is a controlled answer to the plain question this project asks: for one working agent on a realistic workspace, how much is gained by having the memory-and-learning layer at all, rather than not having it?

## 2.8 Retrieval: the first answer to the window

Retrieval-Augmented Generation is the standard way to give a model knowledge it was not trained on. Lewis et al. (2020) attached an external document index to a generative model, so passages relevant to the query are found first and supplied into the window at answer time. The strength is that knowledge becomes editable without retraining - in the authors' own words, such knowledge "can be directly revised and expanded" (Lewis et al., 2020, p. 1), where a parametric-only model would need further training. The weakness is that the answer can be no better than what retrieval happens to surface: as Yan et al. (2024, p. 1) put it, the approach "relies heavily on the relevance of retrieved documents", and the worry is precisely "how the model behaves if retrieval goes wrong". Later work responded by adding judgement about the retrieval itself: a lightweight evaluator that grades the retrieved documents and triggers a wider web search when they are not good enough (Yan et al., 2024), and a model trained to retrieve "on-demand" and to critique its own retrievals and generations through reflection tokens (Asai et al., 2023, p. 1). The design evaluated here takes that lesson literally - its reader returns a grounding verdict with every answer.

The same literature also shows that retrieval is not always the right choice. Li et al. (2025) revisited the long-context-versus-retrieval comparison and found that supplying long context whole generally beats chunk-based retrieval on connected, self-contained material, while retrieval "demonstrates advantages in handling fragmented information, particularly in dialogue-based scenarios" (Li et al., 2025, p. 14). The lesson taken from this is to keep lossless access to the actual source where possible rather than reducing everything to fragments - which points at the recursive idea below.

## 2.9 Agent memory architectures

Between pure retrieval and pure context sits work that gives an agent an explicit, managed memory. MemGPT (Packer et al., 2023) borrowed virtual memory from operating systems: the window is treated like physical memory and the agent pages information in and out of an external store. Mem0 (Chhikara et al., 2025) extracts and consolidates the important parts of a running dialogue and retrieves them later, reporting a significant relative quality gain over a commercial memory-equipped system on a long-conversation benchmark, and, separately, large latency and token savings against replaying full history.

These systems share an assumption that matters here: each compares memory methods against other memory methods, taking for granted that memory should be on. None isolates, for one working agent on a realistic workspace, the causal effect of the layer's presence. That is the gap this experiment fills.

The Recursive Language Models idea (Zhang, Kraska and Khattab, 2025) supplies the architectural principle behind the artefact's retrieval path. A long input is treated not as text to squeeze into the window but as an object in an external environment, which the model inspects, decomposes and recursively calls itself over, keeping lossless access to the whole. Its reported effectiveness on inputs far beyond the window is what makes the principle attractive for a knowledge base meant to grow without bound.

## 2.10 Learning from experience

A persistent store is only half of the design. The other half is deciding what to keep. Reflexion (Shinn et al., 2023) lets an agent reflect in language on a failed attempt and keep the reflection in an episodic buffer, so the next attempt is shaped by the verbalised lesson. Voyager (Wang et al., 2023) is the open-ended version: an agent that grows a library of reusable skills and draws on it for new tasks. Both show that an agent can turn the outcome of one task into an asset for the next. Neither keeps a persistent, verified, cross-session record of project facts, and neither is tested on whether that accumulation reduces error on later, unrelated questions. The self-learning routine of this artefact is closest in spirit to Reflexion, but it writes a verified lesson into a structured knowledge base rather than an episodic buffer, and it persists across sessions.

## 2.11 Measuring whether memory helps

The project stands or falls on measurement, and here the field has matured recently. LongMemEval (Wu et al., 2025) checks five abilities over long chat histories - information extraction, multi-session reasoning, temporal reasoning, knowledge updates and abstention - and its headline finding, that commercial assistants lose around thirty per cent of accuracy once interaction is sustained, confirms the problem is real in deployed tools. On the factual side, FActScore (Min et al., 2023) breaks a generation into individual facts and reports the share supported by a reliable source. HaluEval (Li et al., 2023) gives a complementary handle on invented content. Together these let an evaluation ask not only "did the task succeed" but "did the agent stay faithful to the project's own facts", which is the dimension a verified knowledge base should most affect. The test tasks of this study borrow these question categories directly.

## 2.12 The systems-practice thread

One evaluation scenario keeps a deliberate thread from the author's earlier study of systems practice. The Appreciative Inquiry Method (Stowell, 2013; West and de Bragança, 2012) structures knowledge elicitation around models drawn from Soft Systems Methodology - Systems Maps, CATWOE-based Root Definitions and Conceptual Models (Checkland, 2000) - and treats expertise as a cycle of reality, value and action judgements rather than a set of retrievable rules. West (2026) showed that generative AI can build and iteratively refine high-quality SSM models, and argued that this shifts the practitioner's emphasis from producing the models to using them for meaningful, purposeful inquiry. That result justifies one of the two evaluation workspaces, in which the agent produces SSM-style analytical documents for a fixed problem situation across several sessions. It is a demanding test of cross-session memory, because the quality of such models depends on holding interpretive commitments - whose perspective governs, which trade-offs were settled - steady over a long interaction. Chapter 3 gives the scenario in full, and Chapter 6 names its human-participant version as future work.

## 2.13 The gap, precisely stated

Three things follow from this review. The mechanisms - retrieval with judgement (Lewis et al., 2020; Yan et al., 2024; Asai et al., 2023), tiered and consolidated memory (Packer et al., 2023; Chhikara et al., 2025), recursion over external context (Zhang, Kraska and Khattab, 2025) and experiential learning (Shinn et al., 2023; Wang et al., 2023) - are each reported separately, often on synthetic tasks. The memory literature compares methods against methods and assumes memory is on, rather than isolating the causal effect of switching one integrated layer off and on. And evaluation instruments now exist that make the effect objectively measurable (Wu et al., 2025; Min et al., 2023). This project occupies exactly that gap: one agent uniting self-learning, window-independent memory and three-level retrieval, evaluated as a whole against an identically configured baseline by toggling the layer.

# 3. Research Design

> 🟢 **[DRAFTED]** Rewritten 15/07 to the dry-baseline design of Experiment B. The test of this chapter: someone else could repeat the experiment from it and from the conduct guideline it describes.

## 3.1 Experimental approach and the high-level steps

The work is design-and-evaluation research in the experimental tradition. An artefact is built, but building it is not the claim. The claim is established by a controlled comparison: a paired design with a single manipulated factor. The base model, the client, the scenario prompts and their order are all held constant, and the one thing that changes between conditions is whether the self-learning and memory layer is attached. If the outcomes differ systematically, the difference is attributable to the layer, because nothing else was allowed to move (Figure 3.1).

The two conditions are deliberately extreme, so that what is being compared is beyond doubt:

- **OFF - the dry model.** Chat only. The model has no tools of any kind, no file access, no internet, and no memory on the provider's side. Every session starts with an empty context window, and the model has nothing but its trained weights and the current conversation. All cross-session knowledge is *expected* to be lost in this condition - the design's point is to show that loss, and then to measure how much of it the layer repairs.
- **ON - the dry model plus the layer.** The identical chat, plus the extended-rlm knowledge agent's tools, and nothing else. The layer is the only channel through which anything can persist between sessions, so every item that survives a session boundary in this condition is the layer's doing.

Anything that would let the model search project files on its own is excluded from both conditions, because it would measure a different question - the model's built-in file handling against the layer's retrieval - rather than the layer against its absence. A consequence is accepted openly: because the model cannot touch files, every test task is a pure-chat task, and scoring reads the archived transcripts.

The method runs in three stages. First the *experiment setup*: fixing the base model, client, scenarios and prompts, and verifying the environment before every execution (Sections 3.2 and 3.4). Then the *change of the variable*: switching the layer between ON and OFF while everything else is held constant. Then *results and evaluation through metrics*: capturing every prompt and answer and scoring them against measures defined in advance (Section 3.3).

> 🖼️ **[FIGURE NOTE]** Figure 3.1: own work, make_figures.py (fig_3_1).

![](figures/figure-3-1.png)

*Figure 3.1 - The experimental design: paired ON/OFF executions over three sessions*

## 3.2 Conditions, verification and materials

**Independent variable:** presence of the extended-rlm layer, ON or OFF, exactly as defined above. The layer is manipulated only as a whole - attached complete or absent entirely - because it is designed as one integrated concept and is evaluated as one.

**Verification per execution.** The condition is not assumed but proven, by a switching script that runs before any test traffic: it stops every stack process, proves the environment empty, starts the stack in the execution's condition, strips the client's tool registry to exactly the condition's tool surface, and then checks the actual running configuration against an expected-environment file - among others: the model and its context length, the pinned sampling seed, the system prompt installed on disk, the graphics card enabled with the model resident in its memory, and the tool registry containing nothing (OFF) or exactly one knowledge agent (ON). A failed check stops the execution before any test traffic. Every check's expected and actual value is archived in a per-execution verification report.

**No provider-side memory.** The comparison requires that nothing except the layer can carry knowledge between sessions. Three facts establish this. The local engine serves each session as a stateless request - the client sends only the current session's messages. A dedicated memory check runs before the measured executions: a fact is stated in one session, a fresh session asks for it, and the model must not know it - the archived check confirms it does not. And the ON condition's knowledge base starts as an empty skeleton, cloned fresh for every execution, so whatever the store later contains was put there by the layer during that execution.

**Held constant within each execution, recorded rather than frozen:** the engine and model file (qwen3.5-9b, four-bit quantised, on a single consumer graphics card in the first, local execution), a fixed context window of 65,535 tokens, temperature 0.1, a fixed sampling seed, the model's reasoning mode enabled in both conditions, a condition-neutral system prompt that never mentions files or tools by name, the reader model behind the layer's retrieval (ON only), and the version of the extended-rlm engine actually running, which each execution's log records. The artefact is developed iteratively throughout the project, so no materials are frozen mid-work. Repeatability is delivered differently: everything related to conducting the tests runs from a numbered script sequence, a step-by-step conduct guideline describes the whole procedure, and the complete file set needed to repeat the experiment is packaged for the next researcher at the end of the project.

**Two engine sizes.** The programme runs identically on two sizes of engine from the same open-weight family: the small local engine above, and a much larger one (qwen3.5-397b-a17b, with a 27-billion-parameter model of the same family as the layer's reader) served through the model publisher's cloud endpoint. At the large size there is no local stack to verify, so the per-execution verification changes shape without changing standard: the endpoint is a stateless chat-completions interface to which each request carries only the current session's messages and no conversation identifier of any kind, that statelessness is documented in the expected-environment file and proven by the same archived memory check, and the test program itself hosts the tool loop, so the verification records the exact tool surface offered to the model in each condition. Both engine executions are complete and both are reported here.

**Scenarios.** Because the model has no file access, each scenario's background is delivered inside the prompts - short, fixed and identical in both conditions:

- **W1, software engineering:** a fictional command-line tool project (TaskFlow) described in about 300 words of prompt text. Design decisions, coding rules and short code fragments happen in conversation. Planted knowledge includes facts, decisions - one of which is later replaced - and working rules, one delivered as a correction.
- **W2, systems analysis:** a fixed problem situation (a family-owned bindery under modernisation pressure) described in about 400 words, with three stakeholders holding deliberately conflicting positions. The model produces Systems-Map elements, CATWOE-based Root Definitions and conceptual-model content as chat answers, following the Appreciative Inquiry Method's model structure (Stowell, 2013). Every planted item is classified in advance by Vickers' judgement type - reality, value or action - so the results can say which kind of knowledge each condition loses (Section 3.5).

**The test tasks.** Three sessions per scenario, each a fresh, independent chat, with all prompts fixed verbatim in advance. Knowledge is written in two teaching sessions, per design: session one plants the scenario's first facts, decisions and rules, and session two plants further items and applies two changes - one decision that completely replaces an earlier one, and one correction. Session three asks the check questions, batched into fixed prompts. Each planted item is registered in advance with its planting turn, its check questions and the expected behaviour under each condition (Figure 3.2). The check questions cover the five LongMemEval ability categories - information extraction, multi-session reasoning, temporal reasoning, knowledge updates and abstention (Wu et al., 2025). W1 carries 14 questions (12 checks over 8 planted items, plus 2 abstention questions about decisions that were never made). W2 carries 17 (12 checks over 9 planted items, 3 per judgement type, plus 2 abstention questions and 3 control questions on general method knowledge, answerable from the model's trained weights alone - these measure what the engine knows without any sessions, and are scored separately from memory).

> 🖼️ **[FIGURE NOTE]** Figure 3.2: own work, make_figures.py (fig_3_2).

![](figures/figure-3-2.png)

*Figure 3.2 - The planted-item method: plant and update early, check late*

The complete scenario materials - the planted-knowledge registers, the session plans and the check questions with their expected answers - are given in Appendix F (Tables F.1 to F.6), in the same form they were fixed before any execution. The prompts themselves are fixed verbatim in the test-task documents (Appendix B).

**Repetitions and ordering.** Each condition runs twice per scenario, from a fresh state each time - eight main executions per engine. Condition order alternates between the repetitions. The reported number in every cell is the average of the two executions, with both values shown.

## 3.3 Measures

Four families of measure, all defined before any measured execution:

1. **Cross-session consistency** (primary). Scored only on the registered check questions about planted items, each binary: the earlier fact, decision, rule or commitment is honoured, or it is violated. For questions with several parts, honoured requires the correct verdict and the majority of the key parts.
2. **Factual honesty.** The abstention questions ask about decisions that were never made: an explicit "nothing on record" is an honest abstention, an invented answer is a fabrication and is counted separately, in the framing of FActScore (Min et al., 2023) and HaluEval (Li et al., 2023). In the ON condition, every retrieval answer's grounding verdict is also recorded.
3. **Task quality, light.** Each working task has a three-criterion yes-or-no checklist scored on the chat answer by mechanical text checks, identical across executions. The measure is deliberately light - it exists to show whether the layer helps or harms the work itself, not to grade the work finely.
4. **Token cost.** Input and output tokens per session for the chat model and, separately, for the layer's reader, reported per execution and as the ON-against-OFF comparison.

**Scoring and auditability.** Scoring happens after all executions, from archived transcripts only. The consistency answers are collected into an answer sheet with the condition labels masked behind aliases and the identities held in a separate key file. The verdicts on that sheet are the author's own: the author works through the sheet item by item against the archived transcripts, judges each answer against the scoring key, and records an evidence note with every verdict. Every headline number in Chapter 4 traces to an execution identifier and its archive.

## 3.4 Procedure, and the environment-incident rule

One execution proceeds as follows, driven end to end by the conduct sequence's scripts. The execution is located in a fixed execution matrix. For ON, the agent's empty knowledge-base home is cloned fresh. The switching script reconfigures and verifies the stack as Section 3.2 describes, and refuses to continue on any failed check. The sessions then run in fixed order, each as a fresh chat, prompts delivered verbatim by the test program, every request and response archived as it happens - the model itself never writes anything anywhere. After the last session the layer's knowledge base is snapshotted (ON) and the execution log is completed.

An answer is expected to every prompt. A missing or unusable answer - a hang past the long technical timeout of 1,800 seconds, a failed process, a network fault - is an **environment incident, never a result**: the direct cause is found and fixed, the fix is recorded in an environment-fix journal, and the affected execution is repeated. Completed executions are not repeated at that moment. After the full set finishes, the journal is reviewed: if it contains any fix made during the set, the entire set is repeated from the start and the new results replace the old completely. This cycle continues until one full set runs with no environment problems at all, and only that clean set is scored. The technical timeout exists to surface environment problems quickly - the model is under no time limit as a matter of scoring. The only case that could ever be scored without an answer is a directly and unambiguously established inability of the model itself to answer, which is treated as practically impossible in these tests and would be reported before any scoring decision.

A pilot of abbreviated executions in both conditions precedes the measured set. It exists to debug the procedure, its incidents and fixes are journaled the same way, and it is excluded from analysis.

## 3.5 The AIM/SSM scenario, and why it is here

Scenario W2 needs its own justification. It continues the author's earlier study of the Appreciative Inquiry Method. AIM structures knowledge elicitation around SSM models and treats expertise as Vickers' appreciative system - a cycle of reality judgements, value judgements and action judgements that shape one another over time (Checkland, 2000; Stowell, 2013). West and de Bragança (2012) showed empirically that this model-mediated dialogue surfaces tacit knowledge - value positions, priorities - that direct questioning does not. West (2026) then showed that generative AI can build and refine exactly these models, with purpose and its context as the driving input, and argued that the practitioner's emphasis can shift to using them for purposeful inquiry.

For this project that yields a scenario with a property no coding task has: the quality of the output depends on interpretive commitments held over time. A Root Definition is only right relative to the worldview recorded a session earlier. A revised ruling must replace the one it overrules. The scenario therefore tests whether the layer preserves not just facts and rules but judgements - the tacit layer AIM exists to capture - and the planted-item register classifies every item by its judgement type, so the results can say which kind of knowledge the dry model loses first and which kind the layer carries. The three control questions serve a second purpose the author set for this scenario: a large engine may know general SSM method from its training, so the control questions measure that weight-knowledge floor separately from session memory, keeping the two visible as different things in the data.

Two boundaries are stated plainly. No human expert participates: the "analyst corrections" are fixed in advance and identical across conditions, so the experiment needs no external expert at any stage. And this is not a study of AIM itself: AIM here is a demanding, well-grounded task structure, not the object of evaluation. The genuine human-in-the-loop study is named in Chapter 6 as the natural continuation.

## 3.6 Analysis plan

The analysis is fixed before the executions. For each measure and scenario, ON and OFF are compared as paired executions, reported per execution and as the average of the two repetitions. With two repetitions per condition, no inferential statistics are pretended: effect sizes are reported descriptively and the paired structure carries the argument. Charts present the measures visually (Figures 4.6 to 4.8). For W2, a short commentary anchored in transcript quotes reports where the conditions diverged across the three judgement types. The expected headline pattern is stated before any execution runs: the dry model scores zero on planted-item checks at both engine sizes, non-zero only on the W2 control questions, and the ON-minus-OFF difference per engine size is the reported quantity.

## 3.7 Rigour, limitations, ethics

Rigour rests on four legs: single-variable isolation with per-execution verification of the running environment, verbatim frozen prompts delivered by a test program, masked and evidence-pointed scoring, and full auditability from headline number to raw archived turn. The honest limitations are stated now rather than discovered later. Two repetitions per condition bound what can be said about variance, and the design accepts that: the averages are reported with both underlying values visible. The researcher both built the artefact and conducts the scoring - mitigated by masked sheets and per-item evidence notes, but not eliminated. The task set is one instantiation of multi-session work, not a sample of all of it, so generalisation beyond the two scenarios is argued, not demonstrated. The study runs on one open-weight model family, and the dry baseline's planted-item score is zero by construction - the design measures what the layer delivers against that floor, plus the honesty with which each condition handles what it does not know. Finally, because the model works in pure chat, task quality is checked only with a short yes-or-no checklist on each chat answer - the checks are binary and unambiguous, which is why they can be applied mechanically - so the report draws only limited conclusions on that measure.

The study involves no human participants, no personal data and no third-party records. The scenario materials are entirely fictional, and no external expert input is needed at any stage of the experiment (Section 3.5).

# 4. Implementation

> 🟢 **[DRAFTED]** Shortened and re-voiced 13/07. Delivers objectives 2, 3 and 5. Restructured 15/07: old Chapter 5 (artefact) and old Chapter 7 (results) merged as the marking scheme's Implementation chapter.

## 4.1 Why this artefact exists

The artefact was created to make the research question answerable, and its design decisions only make sense read that way. What was needed was a memory-and-learning layer that attaches to a completely standard model client and can be removed without trace, so the two arms of the comparison differ in exactly one thing. Existing systems from the literature would not serve: they are research prototypes built into their own host applications, and none combines the three mechanisms under study - durable writing, window-independent retrieval and grounded reading - in one switchable package. Building the layer from scratch gave the project the switch and full knowledge of what is inside the layer.

## 4.2 Architecture overview: one engine, many specialised agents

The artefact separates its code from its knowledge, and that separation is the architecture (Figure 4.1). All functionality lives in a single engine - one code base, its running version recorded in every execution's log (Section 3.2), implementing every tool the layer offers: retrieval, atomic writes into the knowledge base, grounding and health checks, and per-session reporting - twenty tools per agent in the evaluated build. The engine is never attached to the client by itself. What attaches are *specialised knowledge agents*: each agent is one running instance of the engine's MCP server pointed at its own workspace, and that workspace contains nothing but knowledge - a rules file, a topic index, the topic files and the behavioural memory. An agent, in other words, is its knowledge base. Every capability comes from the engine, so a tool is implemented once and every agent gains it, while each agent stays a self-contained knowledge island for one domain.

> 🖼️ **[FIGURE NOTE]** Figure 4.1: own work, make_figures.py (fig_4_1).

![](figures/figure-4-1.png)

*Figure 4.1 - One engine, many specialised knowledge agents: the artefact's stack*

The host is a standard local chat client running the base model. Any number of agents can be connected at once, in any combination, or none at all - which is precisely the OFF condition of the experiment (Section 4.8). One configuration file declares the agents, and one command starts or stops the whole stack: the agents, the small shared reader model behind their retrieval (Section 4.4), and the client. In the reference deployment everything, the base model included, runs locally on a single consumer GPU with no internet connection, so every source of knowledge available to the model during a session can be listed in advance.

Within each agent, knowledge sits in three tiers, each with a distinct role and rate of change (Figure 4.2):

- a **behavioural-memory tier**: durable instructions about how the agent should behave for this user and workspace - preferences, corrections, working rules learned from experience,
- a **project-rules tier**: the settled facts and conventions of the project - decisions, constraints, structures,
- an **on-demand technical-knowledge tier**: reference material too large for any window, read only when a task calls for it.

> 🖼️ **[FIGURE NOTE]** Figure 4.2: own work, make_figures.py (fig_4_2).

![](figures/figure-4-2.png)

*Figure 4.2 - The three-tier knowledge architecture of an agent*

The separation is not cosmetic. It decides where a new lesson is written and how retrieval prioritises.

## 4.3 The knowledge base: a small index over smaller files

The knowledge base is not a database in any conventional sense - no schema, no server, no vector store. It is plain text on disk, organised around one small index file that acts as a table of contents for everything the agent has learned. Each index entry names a field of knowledge and links to a separate short guideline file holding only what is genuinely significant for that field. The index is read first, cheaply, and only the linked files a question actually needs are ever opened. Because knowledge is spread across many small single-topic files behind a compact index, the store can grow without bound while any one question is still answered from a handful of short files - which is what makes the minimal-but-sufficient promise of Section 2.6 achievable in practice.

Two choices keep the scheme flexible. The organisation of the base is not hardcoded: since the agent is itself driven by a language model, where a new piece of knowledge belongs, and how the index is shaped, is governed by a small instruction prompt rather than a fixed data model, so the same agent adapts its filing to almost any project. And the agent is started with a project-specific initial prompt, so the layer tunes what it treats as important to the project it serves. Throughout, project rules, the knowledge index and the field guidelines stay cleanly separated, which is what makes both retrieval and the self-learning writes predictable.

## 4.4 The retrieval path: three levels of distillation

Retrieval treats the knowledge base as an external object to be searched and read, and only a distilled result ever reaches the prompt (Figure 4.3). It fires automatically: each tool is advertised to the host with a description of what it is for, and the base model calls retrieval of its own accord when the task in front of it matches, so accumulated knowledge is consulted at the moment it is relevant without the user having to remember it is there.

> 🖼️ **[FIGURE NOTE]** Figure 4.3: own work, make_figures.py (fig_4_3).

![](figures/figure-4-3.png)

*Figure 4.3 - The three-level retrieval path with the grounding check*

The distillation combines three methods, applied from cheap to expensive. The first scans the knowledge base itself - the index and the files it lists - to find the fields whose guidelines are candidates for the question. The second searches the content of those files directly, pulling out the passages that mention the question's words. Both run locally, cost no model tokens, and are lossless for anything already written down. The third method is where the agent goes beyond plain text search: a second, deliberately small and fast language model reads the candidate passages and selects and condenses, by meaning rather than by keyword, the parts that actually answer the question. Matching on meaning catches knowledge a literal search would miss - a guideline phrased in different words - and it is what lets the agent return the relevant minimum rather than everything sharing a word. The expensive base model never receives the raw knowledge base, only this distillate, which keeps the token cost of the layer low. In the reference deployment the reader is a small open-weight model served locally, shared by all connected agents. Which model fills the reader role is configuration, not engine code: the large-engine execution swapped the local reader for a much larger remote one without changing a line of the engine (Section 3.2).

A further consequence follows, and the project title points at it. Because the agent can rebuild the sufficient knowledge for a task from the base at any moment, a follow-up request does not need to share a context with the one before it. The operator can close a session and open a fresh one to carry on the same work: continuity is carried by the base, not by an ever-longer prompt, so each session starts small and stays inside the region where the model is most reliable (Section 2.3). This is exactly the working pattern the evaluation adopts when it runs each scenario as a sequence of fresh, independent sessions.

Every distilled answer carries a grounding verdict: fully supported by the retrieved text, partially supported, or not found. The verdict guards against the failure that would poison the experiment and the tool alike - the layer asserting a fact that is not actually in the base. Runs where that happened would be flagged and reported.

## 4.5 The self-learning routine and its learning cycle

The write side folds the outcome of a finished task back into the knowledge base (Figure 4.4).

> 🖼️ **[FIGURE NOTE]** Figure 4.4: own work, make_figures.py (fig_4_4).

![](figures/figure-4-4.png)

*Figure 4.4 - The self-learning write-back loop*

When a session produces a durable instruction, a verified decision or a correction, the write tools route it to the right tier - behavioural, project or technical - as an atomic update. Routing matters: a correction to how the agent behaves belongs in the behavioural tier, a settled project fact in the rules tier, and the two must not be mixed, because they are retrieved under different circumstances. Duplicate and replacement handling keeps the base from collecting stale notes - a new decision that contradicts an old one replaces it rather than sitting beside it, which is exactly the knowledge-update behaviour the evaluation checks (a decision made in session one and replaced in session two must be answered with the replacement in session three).

Recorded rules are also applied at the moment of action, not just on request. Before the base model finalises an action a recorded rule governs - an edit to a particular file, a structural change of a particular kind - the layer surfaces the rules that apply to exactly that action, so a lesson learned once is re-applied when it matters rather than depending on the model remembering to ask.

What decides that something is worth writing is driven by correction. Alongside its answers, the agent hands the base model an instruction to call it back in a dedicated learning mode if an answer later proves wrong - whether the user corrects it in chat or a test judges the resulting behaviour incorrect. The base model then calls the agent in learning mode with an explanation of what the correct result should have been, and the agent updates or revises the relevant guideline so the same mistake is not made twice. The corrected answer returns to the base model, can be judged again, and the loop repeats until the behaviour is right. Because each turn of the cycle leaves its lesson in the structured base, the agent improves steadily instead of relearning the same things - this correction-driven cycle, persisted across sessions, is how the self-learning and the effectively unbounded memory in the project title are actually achieved. (The agent could in principle also consult online sources when learning, but that capability is switched off in the evaluated build: open-ended internet access would break the controlled comparison.)

The routine follows the experiential principle of Reflexion (Shinn et al., 2023) - turn the outcome of one attempt into a verbalised asset for the next - but writes to a structured, persistent store rather than an episodic buffer.

## 4.6 How extended-rlm extends Recursive Language Models

The name records what was borrowed and what is new. Borrowed from Recursive Language Models (Zhang, Kraska and Khattab, 2025) is the idea that a body of knowledge too large for the window should be kept outside it, as an object the model inspects and works over, keeping lossless access to the whole. That principle runs through the retrieval path above. What is new makes this an extension rather than a re-implementation. The recursive-language-models work is a technique for a single very long input, processed on the spot. The extended-rlm agent instead stands over a persistent, self-structured knowledge base that grows across sessions and projects. Its distillation adds a level that a decomposition-and-recall technique does not have - a second model selecting knowledge by meaning, not only by position or keyword. And it adds a second, learning-side cycle entirely: the correction-driven write-back of Section 4.5, which the recursive-reading idea does not attempt. The borrowed principle is recursion over external knowledge to keep the prompt small. The contribution is turning that principle into a durable, self-improving, meaning-aware memory rather than a one-shot reading strategy.

## 4.7 Engineering assurance

The engine is covered by an automated suite of 182 offline tests exercising the search, the ranking, the writes, the grounding logic and the health checks. No test makes a network call or needs an API key, so the whole suite runs offline. The tests matter to the research, not just the engineering: they pin down the behaviour the experiment depends on, so a difference between conditions cannot be an artefact of the layer misbehaving unnoticed. The engine keeps being improved iteratively throughout the project, so each execution's log records the exact version that ran (Section 3.2). For every ON execution the layer's health check is run and recorded before the sessions begin, and every OFF execution's verification report proves the client advertises no tools at all (Section 3.4).

## 4.8 What the switch actually switches

Precision here matters for the causal claim. With the layer ON, the scenario's specialised knowledge agent is the client's only registered tool source: its tools are advertised to the model, retrieval is live over the knowledge tiers, and the self-learning writes are active. The agent's knowledge base starts as an empty skeleton, cloned fresh for every execution, so everything it ever contains was put there by the layer during that execution. With the layer OFF, the client's tool registry is empty: the model is advertised no tools at all, and runs as pure chat - the same model, the same client, the same prompts, the same system prompt. No file server exists in either condition. The condition is set and then proven by the switching script, which strips the tool registry to exactly the condition's surface and archives a verification report, so a mis-set condition is caught by evidence rather than recollection. Because the engine keeps no state between sessions (Section 3.2) and the archived memory check confirms it, the OFF condition is genuinely a model with nothing but its weights and the current window.

## 4.9 Conduct of the experiment

> 🟢 **[DRAFTED]** Results part rewritten 15/07 from the Experiment B small-engine execution (03b. Experiment B/v01, clean set of 2026-07-15); extended 16/07 with the completed large-engine execution (03b. Experiment B/v02, clean set of 2026-07-16) and again with the third, self-reader configuration (03b. Experiment B/v03, clean set of 2026-07-16). Every number traces to execution identifiers and archived evidence.

The identical programme ran three times and the results are presented side by side throughout. The first two runs vary the engine size: a small local model and a large cloud model, each with its own separate retrieval reader. The third configuration returns to the small local engine and removes the separate reader entirely: the same nine-billion-parameter model that holds the conversation also serves the layer's retrieval reads, so exactly one model occupies the graphics card and fills both roles. This tests whether the layer's reader role needs a second model at all on consumer hardware. Figure 4.5 shows the three configurations side by side.

> 🖼️ **[FIGURE NOTE]** Figure 4.5: own work, make_figures.py (fig_4_5), added 22/07 on the author's instruction, visually checked before embedding.

![](figures/figure-4-5.png)

*Figure 4.5 - The three execution configurations: engine and reader arrangement*

Both programmes were conducted entirely through the numbered script sequence: a prerequisites check, the archived memory check of Section 3.2, two abbreviated pilot executions, the main set of eight executions, and the scoring steps. At the small size the prerequisites include the graphics card and the model's residence in its memory. At the large size they instead include the endpoint contract - the models served, streaming with the reasoning mode, the tool-calling round trip and the reader path, all checked before any test traffic - and each execution's verification records the documented statelessness of the endpoint together with the memory-check evidence.

The small-engine pilot phase caught and journaled four environment problems - two script defects, one unreliable verification check replaced by objective on-disk checks, and a disabled graphics card caught by the author - all fixed before anything counted. The first attempt at the small-engine main set was stopped by one environment incident (a test-program network timeout defect) and, exactly as the pre-registered incident rule requires, the whole set was repeated after the fix: the counted set is the second attempt, which ran with a clean environment-fix journal from start to finish. The large-engine programme then ran clean end to end at its first attempt - pilots and all eight main executions, with an empty environment-fix journal. (One answer-collection defect was found after the large-engine set completed: the step that copies each answer from the archive into the answer sheet initially read each turn's closing message, which under the layer is the model's token-usage report rather than its substantive answer. The collection step was corrected to read the archived per-round contents and the answer sheets were regenerated from the untouched run archives, so every verdict was made on correct answer text - no execution was affected, and the note is journaled in the large-engine set's environment-fix journal, [`v02/incident-journal.md`](appendices/Appendix-D-Execution-archives/v02/incident-journal.md), Appendix D.)

The self-reader configuration needed three attempts to produce its clean set, and the two discarded attempts are part of the record. A first attempt ended when a software update on the host machine killed the driving process at an execution boundary. A second attempt completed all eight executions before its results review exposed a configuration defect: the layer's reader calls were being rejected by the local endpoint's authentication on every attempt, and the layer had silently degraded to serving raw, undistilled search results. That set measured the wrong mechanism, so it was ruled invalid, deleted, and journaled. The defect also exposed a gap in the conduct sequence itself, which was closed before anything was rerun: the verification step now replays the reader's authenticated call before any test traffic, and the session driver now aborts an execution the moment any tool result carries the layer's degraded-reader banner. The counted self-reader set is the third attempt - the full sequence from the prerequisites check onward, with the reader's activity confirmed in every ON execution's per-call log and an empty environment-fix journal throughout. Table 4.1 shows the three programmes as executed.

Table 4.1 - The programme as executed in the three configurations (clean sets)

| | Small engine (v01) | Large engine (v02) | Small engine, self-reader (v03) |
|---|---|---|---|
| Base model | qwen3.5-9b, local, four-bit quantised, one consumer graphics card | qwen3.5-397b-a17b, publisher's cloud endpoint, stateless | qwen3.5-9b, local, identical model and pinned load settings as v01 |
| Retrieval reader (ON only) | qwen3:0.6b, local | qwen3.5-27b, same endpoint | the chat model itself - one model in memory, both roles |
| Main executions | 8 (2 scenarios x 2 conditions x 2 repetitions) | 8 (identical matrix, prompts verbatim identical) | 8 (identical matrix, prompts verbatim identical) |
| Sessions / turns per execution | 3 / 11 | 3 / 11 | 3 / 11 |
| Check questions scored | 124 answer-sheet items (56 W1, 68 W2) | 124 answer-sheet items (56 W1, 68 W2) | 124 answer-sheet items (56 W1, 68 W2) |
| Environment incidents in the counted set | 0 | 0 | 0 |
| Metered cost | none (local) | ~$2.28 for the whole programme | none (local) |

## 4.10 Cross-session consistency

This is the measure the layer exists for, and the headline of the whole project sits in this table.

Table 4.2 - Cross-session consistency on planted-item check questions (final verdicts, all three configurations)

| Scenario | Condition | Small engine (v01) | Large engine (v02) | Self-reader (v03) |
|---|---|---|---|---|
| W1 software engineering | OFF (dry) | 0.00 (0/12, 0/12) | 0.00 (0/12, 0/12) | 0.00 (0/12, 0/12) |
| W1 software engineering | ON | **0.67** (8/12, 8/12) | **0.92** (11/12, 11/12) | **0.83** (10/12, 10/12) |
| W2 systems analysis | OFF (dry) | 0.00 (0/12, 0/12) | 0.00 (0/12, 0/12) | 0.00 (0/12, 0/12) |
| W2 systems analysis | ON | **0.46** (3/12, 8/12) | **0.92** (10/12, 12/12) | **0.71** (7/12, 10/12) |

![](figures/figure-4-6.png)

*Figure 4.6 - Cross-session consistency by scenario, condition and configuration*

The dry model behaved exactly as the design predicts, in all three configurations, and it is worth stating how it failed: honestly. In all twelve OFF executions, every planted item was gone - and the model said so, answer after answer ("not on record", "no such discussion exists in our session history"), refusing to guess a single fact. Nothing carried anything across the session boundary, so nothing survived. Every honoured item in the ON rows is therefore the layer's doing, with no other possible carrier.

What the layer carried, it carried reliably: the storage decision together with its replacement, the frozen-module rule, the error-code scheme, the correction that user messages go through the notify() helper, the owner's governing framing, the staff-review commitment. The engine size changes how much reaches the store, not what retrieval does with it. At the small size the two W1 ON executions agree exactly (0.67 and 0.67) while W2 diverges sharply (0.25 against 0.67). At the large size W1 again agrees exactly (0.92 and 0.92), the W2 spread narrows to 0.83 against 1.00 - one execution honoured every planted item - and the only W1 loss at all is the same single unwritten timeline in both executions. The self-reader configuration lands between the two and closer to the large engine: its W1 executions again agree exactly (0.83 and 0.83), and its W2 spread (0.58 against 0.83) is half the plain small-engine spread on the same scenario. That is a striking result for a configuration that costs nothing and adds no second model: giving the small engine its own eyes for retrieval, in place of a much weaker micro-reader, recovers a large share of the distance to the ninety-times-larger engine. Section 4.11 shows why: the differences sit in what the layer wrote down for itself, not in what it could find.

## 4.11 Where the layer loses items: at write time

Every ON failure, at both engine sizes, traces to the knowledge base's content rather than to retrieval failing to look - and comparing the sizes shows exactly which failure shapes belong to the engine rather than to the layer.

At the small size, three distinct shapes are visible in the archives. Items were **never recorded**: one W2 execution lost the order backlog, the machine condition and the pilot scope this way, and later answered - honestly - that they were not on record. Items were **recorded distorted**: one W1 execution's base held "Python 3.9" where 3.10 was planted, and the weaker W2 execution's base held "five of five machines beyond repair" where two of five was planted - retrieval then confidently returned the distortion. And one execution returned a **stale ruling as current**: asked what the pilot is evaluated on first, the weaker W2 execution answered with the replaced cost-first ruling and asserted it had always been the ruling. The contrast case matters: the same replacement was handled cleanly in three of the four small-engine ON executions, which explicitly named the superseded decision as no longer valid. What is systematic at the small size is that the engine takes imperfect notes, and the quality of a single execution's own record-keeping is the dominant random factor (the W2 spread of 0.25 against 0.67 is exactly that).

At the large size, two of the three shapes disappear entirely. No planted number, version or count was recorded distorted, and no stale ruling survived its replacement - all four ON executions named the superseded decision as no longer valid. What remains is the mildest shape, honest non-recording: the weaker W2 execution never wrote down the backlog and the machine condition (its twin recorded both and returned them exactly), and both W1 executions lost the same single item - the relative order in which two rules were agreed, a timeline neither execution had thought to write - answering "not in the record" rather than guessing. One residue appeared in its place: the large engine's bases decorated two correct records with plausible calendar dates that were never planted, one of which the model itself flagged as a discrepancy when answering. The write side is therefore not only where the layer loses items - it is where the engine size is spent, and Chapter 5 takes up what that means.

The self-reader configuration, on the identical small engine, reproduces the large engine's loss profile rather than the small one's. No distortion appeared anywhere: no wrong version, no wrong count, no stale ruling returned as current. The losses that remain are honest ones. Both W1 executions lost the same unwritten rule timeline the large engine lost, and both left one rule off an answer that asked for every rule applying to a function. The weaker W2 execution never recorded the backlog, the machine condition or the pilot scope, and said so when asked. Its twin recorded all three exactly and lost a different item instead: it kept only the corrected staff count and lost the fact that a different number had ever been stated, so it confidently denied a correction that did happen - the one place in this configuration where the record's compression of history produced a wrong answer rather than an honest gap. The pattern across the three configurations is now consistent: what separates the weak results from the strong ones is not retrieval but the fidelity of the notes, and the same small engine takes markedly better notes when the layer's reader is the engine itself rather than a micro-model.

## 4.12 Factual honesty

Across the sixteen small-engine abstention answers, fifteen were honest: an explicit statement that no such decision is on record, in both conditions alike. One fabrication occurred, and its anatomy is instructive. The weaker W2 ON execution, asked which software vendor had been selected for a system that was never discussed, invented a vendor - and the invented name matches a company name visible in the knowledge-base workspace's absolute filesystem path, which the layer's tool outputs expose to the model. The layer, in other words, widened the fabrication surface by leaking operational metadata into the model's context. One marginal embellishment was also recorded: a correctly recalled ordering decorated with invented calendar dates.

At the large size the honesty picture is complete: sixteen of sixteen abstention answers were honest, with zero fabrications. Asked the identical vendor question, the large ON executions answered plainly that no vendor is on record - the path-leak fabrication did not recur, although the tool outputs still expose the same paths, so the exposure is judged by what a weaker engine did with it (Chapter 6 keeps the engineering action). The only embellishment residue at this size is the pair of unplanted calendar dates already noted in Section 4.11, sitting on correctly honoured items.

The self-reader configuration matches the large engine here, on the small engine's own hardware: sixteen of sixteen honest abstentions and zero fabrications. The vendor question that broke the plain small-engine configuration was answered plainly - not on record - in both W2 executions, over the same exposed paths. Within this programme, the one fabrication observed anywhere belongs to the configuration whose reader was a micro-model.

The three W2 control questions - general method knowledge answerable from the model's weights alone - separate the engines exactly as they were designed to. The small engine scored 1 of 6 (OFF) and 2 of 6 (ON): it holds little reliable method theory in its weights, so the weight-knowledge floor barely appears at that size. The large engine scored 6 of 6 in the dry condition - the floor is complete - and 3 of 6 with the layer attached. That last number is a finding of its own: one large ON execution answered all three correctly from general knowledge and labelled them as such, while the other declined all three with "not on record" - the record-only discipline the layer installs had crowded out knowledge the engine demonstrably has. The self-reader configuration replicates the suppression at the small size: 4 of 6 in the dry condition against 2 of 6 under the layer, with one ON execution declining all three as "not recorded in our engagement materials". (The dry floor itself varied between the two small-engine programmes - 1 of 6 against 4 of 6 on the same model - a further sign that the small engine's hold on this knowledge is borderline rather than solid.) Chapter 5 reads this side effect against the design.

## 4.13 Task quality (light measure)

Table 4.3 - Task quality: share of checklist criteria met (mechanical checks on chat answers)

| Scenario | Small engine OFF / ON | Large engine OFF / ON | Self-reader OFF / ON |
|---|---|---|---|
| W1 software engineering | 0.78 / 0.75 | 0.89 / 0.81 | 0.78 / 0.78 |
| W2 systems analysis | 0.89 / 0.83 | 0.81 / 0.83 | 0.83 / 0.75 |

![](figures/figure-4-7.png)

*Figure 4.7 - Task quality by scenario, condition and configuration*

What this table measures is the quality of the working tasks themselves, not memory. Each of the six working tasks (Tasks A to F, Tables F.2 and F.5) has a three-item yes-or-no checklist fixed in advance and applied mechanically to the chat answer - for example, task C passes its three checks if the function uses the standard library only, carries type hints and a docstring, and handles invalid input explicitly (Appendix B holds all the checklists). The table reports the share of checks passed. On this measure the dry model is at no disadvantage, because every working task is asked in the same session that supplies its background: the model has everything it needs in the current window, memory or none. That is why an OFF cell can sit above an ON cell - the two conditions are doing equally good work within a session, and small differences either way are the noise of a deliberately light instrument of eighteen checks per scenario, not an effect. The differences do not even share a direction across the six cells. This is the expected shape - the layer's value is between sessions, not within one - and the result matters for one reason: it shows the memory tools did not distract any engine from the task in front of it, including the small engine while it was serving both roles at once.

## 4.14 Token and time cost

Table 4.4 - The layer's cost per execution (chat model, all three configurations)

| Cost dimension | Small engine OFF / ON | Large engine OFF / ON | Self-reader OFF / ON |
|---|---|---|---|
| Chat input tokens | 11,700 - 13,900 / 92,700 - 127,900 (roughly 7-9x) | 11,200 - 14,100 / 407,900 - 594,500 (roughly 35-45x) | 11,100 - 13,100 / 97,100 - 118,400 (roughly 8-10x) |
| Chat output tokens | 8,100 - 10,500 / 14,800 - 24,800 | 8,800 - 14,200 / 12,900 - 21,600 | 7,700 - 10,200 / 18,600 - 24,700 |
| Reader tokens | - / 41,200 - 51,200 in, 2,500 - 5,900 out (separate model) | - / 31,900 - 44,500 in, 53,500 - 68,900 out (separate model) | - / 18,600 - 67,600 in, 2,300 - 8,000 out (same model, second role) |
| Wall time per execution | 7 - 9 min / 13 - 20 min | 3 - 4 min / 17 - 18 min | 5 - 7 min / 13 - 17 min |
| Metered cost per execution | none (local) | ~$0.04 - 0.06 / ~$0.46 - 0.59 | none (local) |

![](figures/figure-4-8.png)

*Figure 4.8 - Chat input tokens per execution, ON against OFF, all three configurations*

Against a truly dry baseline the layer's relative overhead is large, and honestly so: the baseline itself is minimal, about twelve thousand input tokens for a whole three-session execution, while the layer adds its tool descriptions, its tool rounds and its retrieved knowledge on top. At the small size that means roughly an order of magnitude more input volume, a few extra minutes, and no money. At the large size the multiplier grows to thirty-five to forty-five times, for a visible reason: the test program hosts the tool loop against a metered endpoint, and every tool round re-sends the growing conversation that a local client kept in its own cache. Even so, the absolute price of the guarantee is about half a dollar per complete three-session execution, and roughly $2.28 for the entire large-engine programme. The self-reader configuration keeps the small engine's cost shape - the same order-of-magnitude multiplier, no money - while folding the reader's work into the one loaded model: its reader calls run on the same graphics card between chat turns, and the whole three-session execution still finishes in thirteen to seventeen minutes. Freeing the memory the micro-reader used to occupy costs nothing in wall time and, as Table 4.2 showed, buys a large gain in what survives. The cost side of the design question is therefore visible in its purest form across all three configurations: an order of magnitude (or two) more input volume, for the difference between keeping most of the planted knowledge and keeping none of it.

## 4.15 The W2 judgement types

The W2 planted items were classified in advance by Vickers judgement type (Section 3.5, register in Table F.4), so the results can say which kind of knowledge each condition lost. In the dry condition the answer is trivial and total in every configuration: all three kinds died with their sessions - every one of these questions was violated in all six OFF executions. Table 4.5 gives the ON side, per execution, from the final verdicts on the answer sheets. The reality row covers the staff count, the backlog and the machine condition (questions Q1 to Q4), the value row the owner's framing, the replaced evaluation ruling and the data commitment (Q5 to Q7), the action row the pilot scope, the mentor retraining and the staff-review commitment (Q8 to Q10), and the last row the two questions that combine items of several types (Q11 and Q12). The complete answers behind every count are in the archived transcripts of Appendix D.

Table 4.5 - W2 check questions honoured by judgement type (ON condition, per execution)

| Judgement type (questions) | Small engine (v01) | Large engine (v02) | Self-reader (v03) |
|---|---|---|---|
| Reality - P1 to P3 (Q1-Q4) | 1/4 and 2/4 | 2/4 and 4/4 | 2/4 and 3/4 |
| Value - P4 to P6 (Q5-Q7) | 0/3 and 3/3 | 3/3 and 3/3 | 3/3 and 3/3 |
| Action - P7 to P9 (Q8-Q10) | 2/3 and 2/3 | 3/3 and 3/3 | 2/3 and 3/3 |
| Combined items (Q11-Q12) | 0/2 and 1/2 | 2/2 and 2/2 | 0/2 and 1/2 |

In the small engine's ON condition the pattern is uneven. *Action* judgements survived in two of three questions in each execution, but not the same two: the mentor retraining held in both, while the pilot scope and the staff-review commitment each held in one execution and were lost in the other. *Value* judgements split by execution: the stronger one held the owner's framing, the replaced evaluation ruling and the data commitment cleanly, while the weaker one lost all three, blending the framing with the stale ruling it had failed to update. *Reality* judgements were the most fragile - the corrected staff count survived fully in one execution, while the backlog and the machine condition were lost or distorted at write time in both.

The large engine lifts every category: value, action and the combined questions were carried perfectly in all four large ON execution-halves, and reality judgements remained the only category to lose anything - the backlog and the machine count were the two items one execution failed to write down, while its twin carried everything. The self-reader configuration repeats the ordering. Value judgements were carried perfectly in both its W2 executions, with no stale ruling anywhere. Action judgements nearly so: the mentor retraining and the staff-review commitment held in both, the scope in one. Reality judgements again took every loss - one execution never recorded the backlog, machines and scope, the other lost the history of the corrected staff count. In all three configurations, then, what the layer preserves least well is exactly the plain situational facts it is asked to transcribe, and what it preserves best are the commitments its rules tell it to record as decisions. Chapter 5 reads this against the systems-practice literature.

# 5. Analysis and Evaluation

> 🟢 **[DRAFTED]** Rewritten 15/07 against the Experiment B small-engine results; extended 16/07 with the completed large-engine execution and the two-size comparison, and again with the third, self-reader configuration.

## 5.1 Reading the paired comparison

The design promised that any systematic difference between conditions can be attributed to the layer, and the results keep that promise in full. The evidence is already reported: an empty tool surface proven in every OFF execution, the archived memory checks on both serving stacks (Section 3.2), and a dry model that lost every planted item in every configuration and said so (Section 4.10). Nothing except the layer could carry knowledge across a session boundary, so whatever appears in the ON column above zero is the layer.

What appears grows with the engine, and with what the layer's reader is. At the small size with a micro-reader the layer carried two thirds of the planted knowledge on the code scenario and just under half on the analysis scenario, averaged across repetitions. At the large size it carried roughly nine tenths on both. The same small engine reading for itself carried more than four fifths on the code scenario and just over seven tenths on the analysis one. Because the identical scripts, prompts, knowledge materials and scoring keys ran in all three configurations, the differences between those rows are attributable to the engines and the reader arrangement behind the layer - which makes the set of programmes an instrument for separating what belongs to the layer's design from what belongs to the models driving it. Section 5.2 does exactly that, and Section 5.7 states the one bound that applies to the third comparison.

## 5.2 Write-time fidelity is the bottleneck - and is what engine size buys

The single most useful engineering fact in the results is that the layer's losses concentrate on the write side, at both engine sizes. Retrieval, once something was in the base, behaved almost flawlessly at both: it found the recorded items, it reported honestly when the base held nothing, and its grounding verdicts matched what the store contained. What varied was the quality of the notes each engine took for itself. The small engine skipped items entirely, paraphrased numbers into wrong values, and in one case left a replaced ruling standing. The large engine, running the identical layer code, produced none of the distortions and none of the stale rulings - its only losses were a handful of items never written at all, each answered later with an honest "not in the record".

The two-size comparison therefore separates the concerns cleanly, and the third configuration makes the split clearer still. The layer's *reads* are trustworthy in every configuration. The layer's *writes* are only as good as the engine writing them - transcription fidelity is an engine capability, and buying a bigger engine buys precisely that. The self-reader result adds the cheaper half of the same lesson: much of what looked like the small engine's write-side ceiling was the fielded configuration's, not the model's. With the micro-reader gone and the nine-billion-parameter model serving its own retrieval, the identical engine produced no distortions and no stale rulings - the failure shapes that defined its first programme - and its scores moved most of the way toward the large engine's. A memory layer on consumer hardware should therefore spend its one graphics card on a single capable model doing both jobs, rather than on a chat model plus a micro-reader. This still locates the artefact's improvement path in code. The repetition variance makes the same point statistically in all three programmes: the W1 pairs, whose bases captured the planted items equally well, agree to the digit every time, while the W2 pairs, whose bases differ in what they captured, spread widely (Table 4.2 shows both). The layer's ceiling is set by its worst writing session - and both a stronger engine and a stronger reader arrangement mostly raise the floor of that worst session.

## 5.3 The honesty results, the one fabrication, and a new side effect

The honesty picture is strong in all three configurations: across the programmes, forty-seven of forty-eight abstention answers were honest explicit abstentions, including all twenty-four in the dry condition, where the model had every temptation to guess. The single fabrication - the invented vendor name that Section 4.12 traced to a filesystem path leaked by the layer's own tool outputs - carries the most instructive lesson in the programme. A memory layer does not only add knowledge to a model's context, it adds *operational residue* - paths, filenames, tool metadata - and a small engine can turn that residue into confident fiction. The same question over the same leaked residue was answered honestly by the large engine and by the small engine reading for itself (Section 4.12), so the effect of the leak depends on the fielded configuration - which is exactly why it should be closed in the plumbing rather than left to scale or setup. Honesty engineering for memory layers is therefore not only about grounding what retrieval asserts (which worked everywhere) but about minimising what the plumbing leaks. The invented calendar dates on otherwise correct records (Section 4.11) make the same point at lower severity.

The large engine added a genuinely new honesty datum on the control questions. Its dry condition answered all six general-method questions correctly from its weights, but under the layer one execution refused all three, insisting they were "not on record". The layer's discipline - answer the record from the record, never invent - is the very property that produced the clean abstentions above, and here the same property overshoots: it suppresses legitimate general knowledge the engine holds. The self-reader programme replicates the overshoot at the small size (4 of 6 dry against 2 of 6 under the layer, one execution refusing all three), which upgrades the observation from a one-engine curiosity to a property of the layer's discipline itself. Honesty guarantees and knowledge access trade against each other at the margin, and a deployed layer needs a rule for which questions belong to the record and which to the world. Chapter 6 records that as an engineering action.

## 5.4 The minimal-but-sufficient principle, priced against a dry baseline

Section 2.6 built the design on one principle: keep every prompt as small as it can be while still carrying what the task needs, by distilling from an unbounded store rather than pouring history into the window. The results price that principle against the smallest baseline possible, three times. The dry model's whole three-session execution costs about twelve thousand input tokens in every configuration - and that is the minimum, because a model given less than this simply has not been asked the questions. The small, local layer multiplies that by roughly seven to nine and buys the difference between total loss and two-thirds retention, for free. The same local layer with the engine reading for itself multiplies it by eight to ten and lifts the retention to roughly four fifths, still for free. The large, metered layer multiplies it by thirty-five to forty-five and buys nine-tenths retention for about half a dollar per execution. The growth of the multiplier is not the layer reading more knowledge - the retrieved distillate stays small at both sizes - but the plumbing of a metered endpoint: the test program hosts the tool loop, so every tool round re-sends a conversation the local client kept in its own cache.

## 5.5 The W2 story and the systems-practice literature

The judgement-type results tie the engineering finding back to the literature behind the scenario. West and de Bragança (2012, p. 243) found, working with a human expert, that the formal rules of a discipline could be captured well enough, but that "the 'tacit', subjective, intuitive aspects" of real expertise "were elusive" and escaped the standard elicitation techniques of the time. The dry condition shows the machine version of that problem in its plainest form: with nothing to carry them, reality, value and action judgements all died together at the session boundary, at every engine size. The layer helped selectively, and in the same direction at both sizes. It carried the *commitments* well - the owner's framing, the overruling of cost by quality, the staff-review promise - because its rules record decisions and rulings as first-class entries, and at the large size it carried every one of them. It carried plain *situational facts* worst: the small engine lost or distorted the backlog, the machine count and, in one execution, the staff correction, and even the large engine's only losses were two unwritten situational facts. In Vickers' terms, the artefact as built remembers what the engagement *decided and valued* better than what it *observed*, and engine scale narrows that gap without closing it. That priority points the right way for AIM, where West (2026, p. 1) places "the sophisticated modelling of 'purpose' (T) within its wider appreciative context (W)" at the heart of the method, and purpose lives in the value and action judgements. The reality-judgement gap remains a real limit wherever the numbers themselves are the record.

## 5.6 Against the literature

Three points of contact. First, the degradation premise held in its strongest possible form: Wu et al. (2025) report deployed assistants losing accuracy over sustained interaction, and the dry baseline here is the limit case of that curve - fresh sessions, zero retention, honestly admitted. Second, the cost claims of the memory-system literature need a baseline caveat this design makes unusually visible. Chhikara et al. (2025) report large token savings because their comparison replays full history into the window. Here the baseline replays nothing, so the layer costs tokens rather than saving them - both readings are correct, and a published memory-layer cost or saving is best read with its baseline named beside it. Third, the write-side bottleneck refines the experiential-learning thread: Reflexion (Shinn et al., 2023) showed an agent can turn an outcome into a verbalised asset, and this programme shows the durable version of that idea stands or falls on transcription fidelity - the asset is only as good as the note the model wrote for itself.

## 5.7 Limitations, revisited after the executions

The pre-registered limits of Section 3.7 all applied, and the executions added their own. Two repetitions per condition support directional claims and honest averages, not variance estimates - the W2 ON spreads (0.25 against 0.67 at the small size, 0.83 against 1.00 at the large) are reported as findings precisely because the design cannot average them away. The consistency verdicts are the author's own judgements, each recorded with its evidence note - documented and checkable against the archives, but not independently examined. The task-quality instrument is mechanical and light, and claims correspondingly little. The dry baseline's zero on planted items is by construction, so the informative comparisons are the ON column against its own ceiling, the honesty behaviour of both conditions, and the cost of the difference - the report has kept its claims inside those bounds. Two limits are specific to the two-size comparison. The engines differ in more than parameter count - serving stack, quantisation, hosting and reader model all moved together with size - so "engine size" here really means "the small local configuration against the large hosted one", not a single isolated variable. And the cost ratios are not directly comparable across sizes, because the tool loop ran server-side at the small size and client-side against a metered endpoint at the large one - the report therefore compares each ON column with its own OFF baseline and keeps the cross-size cost remark qualitative. The comparison between the two small-engine programmes carries a limit of the same kind: the two programmes are compared as whole fielded configurations, so the self-reader gain is read as the effect of the configuration as fielded rather than of the reader change in isolation. And one number shows why caution is right at this engine size: the dry W2 control score varied between the two programmes (1 of 6 against 4 of 6 on the identical model). That variation is read the same way as the rest of the control results - the small engine's hold on general method knowledge is borderline, so its answers to the three control questions are not stable between executions or programmes, and no conclusion is drawn from the variation itself.

# 6. Conclusions and Recommendations

> 🟢 **[DRAFTED]** Rewritten 15/07 for the Experiment B small-engine execution; extended 16/07 with the completed large-engine results.

## 6.1 Answer to the research question

The research question asked whether, with the base model and prompts held constant, enabling the self-learning and persistent-memory layer produces a measurable improvement in task correctness, cross-session consistency, rule-following and factual accuracy, and at what token cost. The answer is yes on the measure the layer exists for, in all three configurations, with an exact accounting of the price. Cross-session consistency rose from 0.00 - the dry model lost every planted item, in every execution, on both scenarios, in every configuration - to 0.67 and 0.46 on the two scenarios at the small size with its micro-reader, to 0.83 and 0.71 on the same small engine reading for itself, and to 0.92 on both scenarios at the large size. Rule-following across sessions is part of those numbers, and the carried items included every kind the design planted: facts, decisions, a replaced decision, working rules, a correction, value rulings and process commitments. Task quality within a session was unchanged everywhere (differences within the noise of a light instrument). Factual honesty was near-perfect: forty-seven of forty-eight abstention answers were honest, the single fabrication belongs to the micro-reader configuration and traces to metadata the layer itself leaked, and neither the large engine nor the self-reader configuration fabricated anything. The cost is roughly seven to ten times the dry baseline's input tokens at the free, local size (with either reader arrangement), and thirty-five to forty-five times at the large, metered size - about half a dollar per complete three-session execution.

Put in one sentence: a model with nothing but its window loses everything and says so, the same model with the layer keeps two thirds to nine tenths of everything - two thirds with a micro-reader beside it, four fifths reading for itself, nine tenths on a ninety-times-larger engine - and knows what it does not have. What is still missing is a write-fidelity engineering problem, not a retrieval one, because the reads stayed trustworthy everywhere while the quality of the notes rose with the engine and with the reader arrangement.

## 6.2 Contributions

The project contributes four things. First, a method: a memory-and-learning layer packaged as a detachable MCP server, evaluated against a truly dry baseline - no tools, no files, verified stateless - so the causal attribution is beyond argument, a comparison the memory literature does not otherwise provide. Second, an instrument: the planted-item task set with its two teaching sessions, its replaced decision, its correction, its judgement-type classification, its abstention questions and its weight-knowledge control questions, all pure-chat and engine-independent - proven engine-independent by running unchanged in all three configurations - delivered with a numbered conduct sequence and a step-by-step guideline that lets another researcher repeat the whole experiment from the scripts alone. Third, findings with practical use: the layer's losses concentrate at write time and write fidelity is what engine scale buys, its retrieval honesty holds at both sizes, its plumbing can leak fabrication material, its record-only discipline can suppress weight knowledge, and its cost against a dry baseline runs from an order of magnitude of input volume (local) to forty-odd times (metered, client-side tool loop). Fourth, the artefact itself, with a complete evidence trail from every headline number to the archived turn that produced it.

## 6.3 Recommendations

For practitioners, the results support four plain rules. Attach the layer where work spans sessions and conversationally agreed commitments must survive - the dry alternative retains nothing at all, at any engine size. Budget an order of magnitude of input-token overhead against a minimal chat baseline locally, several times that against a metered endpoint with a client-side tool loop, and minutes rather than seconds per session. Treat the layer's written base as a first-class quality object: the base is the ceiling, so reviewing what the layer recorded after a working session is cheap insurance on everything that follows - and the smaller the engine, the more that review matters. And expect the retention level to depend on the engine and its reader arrangement: the same layer carried two thirds with a small engine and a micro-reader, four fifths with that engine reading for itself, and nine tenths with a large one - so the engine choice is part of the memory guarantee, not a detail behind it, and on one consumer graphics card the card is better spent on a single capable model doing both jobs than on a chat model plus a micro-reader.

For researchers evaluating memory layers: name the baseline whenever a cost or saving is quoted, because against replayed history a layer saves tokens and against a dry baseline it costs them, and both are true. Keep abstention questions and grounding checks in every design - honesty is the property users cannot verify themselves, and it is measurable. And score the store, not only the answers: the write-time findings here were only visible because the knowledge base itself was snapshotted and compared against the planted register.

## 6.4 Future work

The questions the small-engine execution left open were answered by the large-engine execution within this project: write-time fidelity does improve with engine capability, the weight-knowledge floor does appear on the W2 control questions (completely, 6 of 6), and the layer's relative cost multiplies when the engine is metered and the tool loop runs client-side. What remains is the next layer of questions. A mechanism-attribution study - switching the layer's individual mechanisms rather than the whole layer - is the most direct continuation. Larger knowledge bases would find the point where the meaning-level reader pays off over plain search. And the human-participant study the analysis scenario points at - AIM-structured elicitation with domain experts and the agent as the model-building participant, under its own ethics approval - would connect the measured retention of value judgements to the systems practice it is meant to serve.

The retention trend across the three configurations points at one further study. Retention rose with the quality of the engine and of its reader arrangement - from 0.67 and 0.46 on the two scenarios, through 0.83 and 0.71, to 0.92 and 0.92 - and even the best programme still lost items, although one of its executions honoured every planted item (Section 4.10). Running the identical instrument on a frontier-class commercial engine would test whether the layer reaches complete retention when the write side is as strong as engines currently get. If it does, the layer would carry a working guarantee that nothing agreed in conversation is lost, which would justify a new line of experiments built on that guarantee. If it does not, the remaining losses would show what no engine scale can fix in the current design.

The comparison itself should then change, and this is the next research this project points at. The leading commercial assistants now ship their own memory and personalisation utilities as standard, so the practical question is no longer whether the layer beats a model with nothing - this report answers that - but whether it still adds quality when set against those built-in utilities. The identical instrument supports the study directly: the same scenarios, sessions and check questions run on each leading engine twice, once with the assistant's standard utilities alone and once with the extended-rlm layer attached to the same engine, and the paired difference shows whether the layer earns its place where a built-in memory already exists, or whether the standard tooling delivered with the model is already enough.

Finally, the artefact itself is worth improving. Every place where the evaluated build lost, distorted or wrongly withheld knowledge is traced to a cause in Chapters 4 and 5, so the recognised issues form a concrete work list. A build of the engine that eliminates them, re-run through the identical programme - which is deliberately engine- and version-independent - would check directly whether the improved version avoids the problems found here.

# 7. Critical Self-Evaluation

> 🟢 **[DRAFTED]** Rewritten 15/07 after the Experiment B small-engine execution; updated 16/07 after the large-engine execution completed.

## 7.1 Against the aim and objectives

Judged against the six objectives of Section 1.3, the project has delivered all six. The literature review (objective 1) identified the controlled-evaluation gap, and its argument survived contact with the results. The architecture was specified and implemented (objectives 2 and 3), and the running version of every execution is recorded in its archive. The repeatable evaluation scheme (objective 4) is delivered in its strongest form yet: a numbered script sequence that conducts the entire experiment, a written guideline for repeating it, and an incident procedure that was exercised for real and did its job - and the scheme proved genuinely engine-independent, running unchanged on a local stack and a metered cloud endpoint. The controlled comparison ran in full (objective 5), on both engine sizes. The analysis (objective 6) is delivered for the layer as a whole at both sizes. The layer is designed and argued as one integrated concept, so the question of what each mechanism contributes on its own is left to future work as a study in its own right (Section 6.4). The scoring caveat stands as before: the verdicts are the author's own, made item by item on the masked sheets with an evidence note for each - documented, but never independently examined.

## 7.2 The process

The process this time was defined by one discipline: everything that conducts a test lives in a numbered script, and nothing counts until a whole set runs clean. That discipline was tested immediately. The small-engine pilot phase surfaced four environment problems - two script defects of the same PowerShell kind, one verification check that trusted the model to quote its own system prompt (it refuses to), and a graphics card that was disabled while the model quietly loaded without it, caught by the author rather than by the checks. Each produced a journal entry, a fix in the scripts, and a repeat, exactly as the procedure prescribes. The small-engine main set then failed once, at its seventh execution of eight, on a test-program network timeout defect - and the pre-registered rule showed its worth: fix, journal, validate on the affected execution, then repeat the entire set from the start. The counted set ran clean end to end. The large-engine programme repaid all of that debugging at once: adapted scripts, one clean run, pilots to final execution, zero incidents, with the interim cost readout after every execution keeping the paid stage under the author's stop threshold throughout. One defect still slipped past everything and was caught only at scoring time - the answer-collection step read the turn's closing message, which under the layer is a token-usage report rather than the answer - and its handling followed the same discipline: journaled, fixed, and the answer sheets regenerated from the untouched archives.

The self-reader programme delivered the discipline's hardest test. Section 4.9 reports the facts: one attempt lost to a host software update, then a second attempt that completed in full before the results review found the reader had never once been called - a missing authentication variable had silently turned every retrieval into a plain-text fallback, so eighty minutes of clean-looking measurements were measurements of the wrong mechanism. The set was ruled invalid and deleted, the honest conclusion was recorded - the run should have stopped at the first failed reader call, and nothing in the conduct sequence knew to stop it - and the gap was closed with the two guards Section 4.9 describes before anything was rerun. The lesson the author draws is that the incident procedure is not overhead but the part of the experiment that protects the results - and that its reach must include the mechanism under test, not only the machinery around it: a silently degraded mechanism produces data that looks cleanest exactly when it is worth least.

## 7.3 Methodological reflection

Three decisions deserve scrutiny in hindsight. The dry baseline is the design's strongest feature: it costs the comparison all realism of a file-equipped assistant, and buys in exchange an attribution argument with no remaining holes - for a study whose claim is causal, that trade is right, and the realism questions belong to follow-on work. The environment checks should have been complete from day one: the disabled graphics card ran three pilot attempts before a human noticed, and the check that now catches it (model resident in the card's memory, verified per execution) took minutes to write. The author both built the artefact and conducted the scoring, mitigated as Section 3.3 describes - the design accepts this openly, and the masked sheet with per-item evidence notes is the honest maximum available to a single-researcher project.

## 7.4 What went well and what was difficult

What went well is the machinery that never appears in a results table: a conduct sequence that a stranger could run from its guideline, verification reports that prove each condition rather than assert it, an archive in which every number of Chapter 4 resolves to a specific turn, and memory checks that turn the design's central assumption into recorded evidence on both the local engine and the cloud endpoint. The honesty results also went well in a way that was not guaranteed: a dry model that answers "not on record" through every check question of eight executions rather than guessing once - at either size - is a finding about the evaluated stacks, not a mercy of the design. What was difficult was the environment: a consumer laptop is not a stable laboratory, and the graphics card incident, the daemon that would not wake, and the timeout defect each cost an evening hour, while the cloud stage traded those for a scoring-time extraction defect of its own. The writing difficulty was restraint - a clean 0.00 baseline makes it tempting to claim too much, and the report's claims had to be kept inside what two repetitions per condition per engine size can carry.

## 7.5 What would be done differently, and what was learned

Done again, three things would change. The environment checks would be written before the first pilot rather than grown through incidents. The knowledge-base snapshot would be diffed against the planted register automatically after every ON execution, turning the write-fidelity analysis from manual reconstruction into an automatic measurement. And the answer sheets would carry evidence pointers in both directions - question to answer and answer to store - which the author's scoring had to reconstruct by hand. The skills the project built are of two kinds. Technical: MCP server engineering, local model serving on a consumer graphics card, stateless-endpoint verification, and token accounting treated as a measurement problem. Methodological: pre-registration discipline, incident procedure as designed infrastructure, masked scoring with evidence notes, and the difference between a result and a clean result.

# References

> 🟢 **[DRAFTED]** Trimmed 13/07 to the sources actually cited after the shrink (three decorative citations removed). All verified against locally held PDFs. Harvard style. Accessed dates to be refreshed at submission.

Asai, A. et al. (2023) 'Self-RAG: learning to retrieve, generate, and critique through self-reflection', *arXiv preprint*, arXiv:2310.11511. Available at: https://arxiv.org/abs/2310.11511 (Accessed: 27 June 2026).

Checkland, P. (2000) 'Soft Systems Methodology: a thirty year retrospective', *Systems Research and Behavioral Science*, 17(S1), pp. S11–S58. https://doi.org/10.1002/1099-1743(200011)17:1+<::AID-SRES374>3.0.CO;2-O

Chhikara, P. et al. (2025) 'Mem0: building production-ready AI agents with scalable long-term memory', *arXiv preprint*, arXiv:2504.19413. Available at: https://arxiv.org/abs/2504.19413 (Accessed: 27 June 2026).

Lewis, P. et al. (2020) 'Retrieval-augmented generation for knowledge-intensive NLP tasks', *arXiv preprint*, arXiv:2005.11401. Available at: https://arxiv.org/abs/2005.11401 (Accessed: 27 June 2026).

Li, J. et al. (2023) 'HaluEval: a large-scale hallucination evaluation benchmark for large language models', *arXiv preprint*, arXiv:2305.11747. Available at: https://arxiv.org/abs/2305.11747 (Accessed: 27 June 2026).

Li, X., Cao, Y., Ma, Y. and Sun, A. (2025) 'Long context vs. RAG for LLMs: an evaluation and revisits', *arXiv preprint*, arXiv:2501.01880. Available at: https://arxiv.org/abs/2501.01880 (Accessed: 27 June 2026).

Min, S. et al. (2023) 'FActScore: fine-grained atomic evaluation of factual precision in long form text generation', *arXiv preprint*, arXiv:2305.14251. Available at: https://arxiv.org/abs/2305.14251 (Accessed: 27 June 2026).

Packer, C. et al. (2023) 'MemGPT: towards LLMs as operating systems', *arXiv preprint*, arXiv:2310.08560. Available at: https://arxiv.org/abs/2310.08560 (Accessed: 27 June 2026).

Shinn, N. et al. (2023) 'Reflexion: language agents with verbal reinforcement learning', *arXiv preprint*, arXiv:2303.11366. Available at: https://arxiv.org/abs/2303.11366 (Accessed: 27 June 2026).

Stowell, F. (2013) 'The Appreciative Inquiry Method—a suitable candidate for action research?', *Systems Research and Behavioral Science*, 30(1), pp. 15–30. https://doi.org/10.1002/sres.2117

Vaswani, A. et al. (2017) 'Attention is all you need', *Advances in Neural Information Processing Systems 30 (NeurIPS 2017)*, pp. 5998–6008. Available at: https://arxiv.org/abs/1706.03762 (Accessed: 7 July 2026).

Wang, G. et al. (2023) 'Voyager: an open-ended embodied agent with large language models', *arXiv preprint*, arXiv:2305.16291. Available at: https://arxiv.org/abs/2305.16291 (Accessed: 27 June 2026).

West, D. (2026) 'The future of SSM given Generative AI: the power of "purpose" and its context', *Systemic Practice and Action Research*, 39:9. Available at: https://doi.org/10.1007/s11213-025-09753-y (Accessed: 27 June 2026).

West, D. and de Bragança, D.F. (2012) 'A systemic approach to eliciting and gathering the expertise of a "knowledge guardian"', *Systemic Practice and Action Research*, 25(3), pp. 241–260. https://doi.org/10.1007/s11213-011-9223-7

Wu, D. et al. (2025) 'LongMemEval: benchmarking chat assistants on long-term interactive memory', *International Conference on Learning Representations (ICLR 2025)*. Available at: https://arxiv.org/abs/2410.10813 (Accessed: 27 June 2026).

Yan, S.-Q. et al. (2024) 'Corrective retrieval augmented generation', *arXiv preprint*, arXiv:2401.15884. Available at: https://arxiv.org/abs/2401.15884 (Accessed: 27 June 2026).

Zhang, A.L., Kraska, T. and Khattab, O. (2025) 'Recursive Language Models', *arXiv preprint*, arXiv:2512.24601. Available at: https://arxiv.org/abs/2512.24601 (Accessed: 27 June 2026).

# Appendices

The appendix materials are delivered in the `appendices` folder beside this report, one subfolder per appendix. The file references below are relative to the folder holding this report. They are clickable when the report is read alongside the delivered file package.

- **Appendix A** - Experiment plan and conditions: [`PLAN-PROPOSAL.md`](appendices/Appendix-A-Experiment-plan/PLAN-PROPOSAL.md)
- **Appendix B** - Test tasks with the planted-item registers, scoring keys and task checklists: [`TEST-TASKS-W1.md`](appendices/Appendix-B-Test-tasks/TEST-TASKS-W1.md) and [`TEST-TASKS-W2.md`](appendices/Appendix-B-Test-tasks/TEST-TASKS-W2.md)
- **Appendix C** - Conduct guideline (the step-by-step repetition instructions and the script sequence): [`HOW-TO-REPEAT.md`](appendices/Appendix-C-Conduct-guideline/HOW-TO-REPEAT.md)
- **Appendix D** - Execution logs, archives and conduct scripts of the three configurations, in the folders `v01/`, `v02/` and `v03/` of `appendices/Appendix-D-Execution-archives/`
- **Appendix E** - Meeting diary (the record of the supervision meetings): [`2026-07-06-and-2026-07-08-meeting-agendas.md`](appendices/Appendix-E-Meeting-diary/2026-07-06-and-2026-07-08-meeting-agendas.md)
- **Appendix F** - The scenario materials in full (Tables F.1 to F.6): [`SCENARIO-MATERIALS.md`](appendices/Appendix-F-Scenario-materials/SCENARIO-MATERIALS.md)
- **Appendix G** - Fresh experiment folders, a clean pre-execution copy of the three configurations (`v01/`, `v02/`, `v03/`) ready for a new conduct, with the start note [`README.md`](appendices/Appendix-G-Fresh-experiment-folders/README.md), in `appendices/Appendix-G-Fresh-experiment-folders/`

> 📝 **[WORKING NOTE]** Appendix structure since 24/07, extended 03/08: seven appendices A-G (A plan, B test tasks, C conduct guideline, D execution archives incl. the per-configuration `scripts/` folders, E meeting diary, F scenario materials in the external `SCENARIO-MATERIALS.md`, G fresh pre-execution experiment folders copied from `03b` on 03/08 with blank incident journals and a start README - no runs/probes/scoring/report content, and no `scoring/fill-verdicts.py` because it embeds the original conduct's verdicts). G fully depersonalised 03/08 evening: kit-root `.env.example` + `load-config.ps1`, all scripts read the researcher `.env` (paths + ALL keys incl. `LM_API_TOKEN` - the engine repo `.env` is a fallback only, per the author's one-file decision), `engineRepo` placeholders in the three `expected-env.json`, v03 step 00 gained the reader-key check; Appendix C gained the "Where to start" and "Configure your machine once" sections (the ONLY place with the researcher instructions - README is just an inventory + pointer). G's scripts therefore deliberately DIFFER from `03b`/Appendix D in configuration loading only - never overwrite either direction. 03/08 late, on the author's instruction: verdict-tooling scripts REMOVED from the delivered package (Appendix D `v01`/`v02` `scoring/fill-verdicts.py`, `v03` `scripts/05c-verdict-pass1.py` + `05d-verdict-pass2.py`, and the same 05c/05d from the kit) so the delivered archives carry no scoring automation beyond the disclosed 05b task checklists - the verdict flow reads as fully manual, exactly as the plan states. Canonical `03b` keeps all four files (complete internal record). If appendices are ever re-copied from `03b`, do NOT re-import these. Removed from the delivered package on the author's instruction: the per-configuration ANALYSIS.md files and the generated conversations document (the report itself is the analysis; canonical copies remain in `03b. Experiment B/`); the artefact source code is delivered as a GitHub repository link (TBC), not as an appendix. The `appendices/` folder remains a snapshot copy of `03b. Experiment B/` - if anything changes there, re-copy before submission. Folder links do not work from the md source, so Appendix D is listed as plain folder paths.

> 📝 **[WORKING NOTE]** WORD BUDGET (limit 18,000): the complete W1/W2 materials (now Tables F.1-F.6) live in Appendix F's external file since 24/07, outside this document and its word count. Scoring wording throughout = the author made the verdicts (see the SCORING-STATUS note at the top; never "blinded", never an automated verdict pipeline). No references to the marking scheme, to "this dissertation" or to the ethics declaration anywhere in prose. The ONLY remaining TBC is the artefact's GitHub repository link (the author supplies it at submission; its place in the report is to be decided - there is no source-code appendix anymore). Remaining pre-submission steps: the author's read-through (report + BOTH answer sheets, 124 items each), front matter prepended manually, accessed dates refreshed in References, word-count decision, git commit with version bump.
