# Repository and Project Guidelines

## Why does this file exist?

Because I do not want to maintain distinct `CONTRIBUTING.md`, `AGENTS.md`, and `AI_POLICY.md` files (the last of which does not seem particularly necessary).

Nekor is maintained as a closed-contribution project, and external contributions *will not* be accepted upstream, regardless of whether they are written by a human, a machine, or a mindless, vibe-coding fucktard.

## Why?

I thought of portraying this as some kind of quest of "owning the architecture", but no, it's much simpler than that:

* The probability of you even attempting to contribute is rather low.

* You may not even possess the technical competence required to contribute meaningful logic to Nekor.

* You may believe you are skilled enough, but your code may lack the style, order, or architectural elegance required in Nekor.

Note that this is not an attempt to sound egotistical: I just don't want what is most likely dogshit code in my repository.

So, *please don't contribute code* to _my project_ :)

## Could this be changed in the future?

Perhaps so, but it would require Nekor to be substantially more complete, so that it does not lose its principal architectural essence.

# AI Disclosure

AI-authored code is not, and will never be, permitted in this repository. This is not ideological, as it is due to the low-quality code that is often generated, which requires extensive prompting to achieve a proper result, and the uncertain copyright and provenance issues associated with AI-generated code, particularly when commingled with a human-authored codebase.

## On AI-assisted reviews

Sometimes, having a language model go over an implementation to identify obvious or potential bugs is still beneficial, particularly on a project where the bus factor is *one*, and some bugs may be easily overlooked. This is also important for architectural risk assessment, as we don't exactly have an Intel engineer on call for consultation.

## General AI-assistance Guidelines

AI agents are permitted to inspect and analyze the repository for correctness, security, performance, and architectural concerns.

The following restrictions apply:

* AI agents must not create, modify, or delete repository files, generate patches or commits, or author implementations.

* AI-assisted reviews may identify bugs, vulnerabilities, architectural deficiencies, and potential improvements. Explanations and recommendations are permitted, but generated implementations are not.

* Any findings, recommendations, or technical claims produced by an AI model are advisory and must be independently verified by the maintainer.

* AI-generated code must not be incorporated into Nekor, regardless of whether it has been reviewed, modified, or otherwise adapted by a human.

The purpose of AI assistance is to challenge assumptions and identify potential issues, not to delegate implementation or architectural authority. All implementation decisions and code authorship remain exclusively with the maintainer.
