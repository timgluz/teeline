---
title: "The passkey is the account: replacing our auth provider with self-hosted WebAuthn"
description: "How teeline swapped a hosted sign-in page and OAuth buttons for one-tap passkeys, self-hosted sessions, and API keys you can actually manage — and the honest trade-offs of doing it at zero users."
pubDate: 2026-08-15
tags: ["auth", "webauthn", "passkeys", "cloudflare"]
draft: true
---

[teeline](/) is a browser-based Traveling Salesman Problem solver. Upload a `.tsp` file, pick from
18 algorithms, get a tour back — all in-browser via WebAssembly. It also has an API, and that API
needs keys, and those keys need accounts. This post is the story of how I ripped out the hosted
identity stack I'd been using and replaced it with passkeys running on my own infrastructure — in
one day, at zero users, with my eyes open about what I gave up.

## The hook: identity was literally another website

The old flow for getting an API key went like this: you visited the API-key page on teeline, and
it sent you to *another website* — a subdomain owned by my auth provider — to sign in. GitHub,
Google, or email, take your pick. Then you came back, and only then could you generate a key.

Think about what that means for a developer who just wants to call an API: the thing they're
trying to do — "give me a secret so my script can talk to your solver" — was gated behind a
three-party detour through someone else's product. The identity stack *was* someone else's
product. Every sign-in button, every session, every "forgot password" flow, even the page they
looked at while signing in, belonged to a vendor I paid and whose roadmap I didn't control.

That's not inherently wrong. It's the default way to do auth in 2026. But it sat uneasily with me,
and over time the reasons to keep it eroded one by one.

## Why the hosted provider was right at first

Let me be fair to my past self: signing up for a hosted auth provider was the correct call when I
made it. I got OAuth buttons for free. GitHub sign-in, Google sign-in, email — working on day one
with zero auth code in my own codebase. No password hashing, no session management, no security
review of my own crypto. Just a script tag and a redirect URL.

The rule I was following is a good one: *use an identity provider until you can't.* Auth is
genuinely hard to get right — credential storage, session invalidation, recovery flows, the whole
attack surface. A specialist vendor is a reasonable answer for most projects, most of the time.

So the question was never "is hosted auth bad?" It was "when does the deal stop being worth it?"
For teeline, three things pushed me over the edge, and none of them is exotic.

## The three pushes

**First: the product isn't monetized, and the features I wanted sat behind a paywall.** The
interesting auth features — the ones that would have made sign-in feel native, passkeys chief
among them — were on the paid tier of my provider's pricing. Not expensive, exactly, but the
principle was absurd: I was being asked to pay for login buttons on a free, non-commercial
project. The provider's economics assume you're a SaaS with revenue or a venture trajectory.
teeline is neither.

**Second: the OAuth provider list is a maintenance matrix, not a feature.** Every provider you
enable — GitHub, Google, email, maybe GitLab — is a little piece of someone else's system you've
agreed to keep working. They change their flows, they deprecate scopes, they rotate their own
requirements, and each one is a support channel you didn't ask for. The more I thought about it,
the more the "free OAuth buttons" looked like deferred maintenance wearing a feature's clothes.
For a single developer, that's a tax on the thing I actually want to do, which is improve the
solver.

**Third — and this is the one that reframed everything — the actual job is "a developer wants an
API key," not "an account."** When someone lands on the API-key page, they don't want to *join*
anything. They don't want a profile, a dashboard, a community. They want a secret string that
makes their curl script work. The entire ceremony — choose a provider, click through consent
screens, come back, verify email, set a password — exists to support a moment that should be one
tap.

Once I saw it that way, the whole shape of the problem changed. What if the account *was* the
one tap?

## The idea: the passkey is the account

Passkeys are WebAuthn, which has been around for years and is now genuinely everywhere: built
into the browser, the OS, and every password manager worth using. The ceremony is handled by
infrastructure the user already has — their device's secure enclave, their password manager —
and the server never sees a password, a secret, or anything it could leak. What the server stores
is a single public key and a counter.

Here's the reframe: if a passkey authenticates the user, and the passkey lives in their password
manager, then **the passkey is the account.** There is no separate "user account" with a username
and a password to recover. There's a public key, a counter, and an identity derived from the
credential itself. The user's tap *is* their identity. No username to forget, no password to
reset, no email to verify.

The clincher was timing: teeline has zero users. Not "a few." Zero. That means I could make big,
irreversible-feeling trade-offs without hurting anyone, because there was no one to hurt. A
migration like this at 100,000 users would be a project with a rollout plan and a support queue.
At zero users, it's a weekend — and in my case, it was a day.

That day had a shape worth describing, because it's the shape of the trade-off in miniature: nine
small pull requests, each one squash-merged and deployed green before the next started. Data
layer, then the ceremonies, then API keys, then the sign-in UI, then the Rust API's verifier,
then end-to-end tests, then hardening. Each step was independently shippable, which is exactly
what you can afford when nothing is live yet. The end-to-end suite even drives a virtual
authenticator in a real browser, so the one-tap flow is exercised the way a user actually
experiences it — which is how the envelope bug got caught before it shipped.

## The shape of the new flow

I want to describe the result the way a user experiences it, because that's the point of the
whole exercise.

You visit the API-key page. There's a button: **Sign in with passkey.** You tap it. Your browser
or password manager asks — in whatever way it normally does — to authenticate. That's the entire
sign-in. No provider chooser, no consent screens, no redirect to another domain. One gesture.

Now you're signed in. You click **Generate API key.** A secret appears, shown exactly once, with
the honest reminder to save it in your password manager — where it will sit next to the passkey
that got you in. If you refresh the page, the secret is gone; only a hash of it remains on the
server, and there is no way to view it again. You can list your keys, revoke one instantly, or
mint a fresh one whenever you want.

Two different credentials, doing two different jobs, living in the same place:

- the **passkey** authenticates *you* — the human — to the site;
- the **API key** authorizes *your scripts* to call the solver.

That separation is the whole product: the human taps, the machine sends a header, and the same
password manager happens to hold both. It's the first time the auth story of this project has
felt like it was designed for its actual users.

## The honest ledger: what I gave up

I don't want this to read as a victory lap, because a passkey-first, self-hosted setup involves
real, deliberate sacrifices. Here's the ledger, with the debits first.

**Passkey loss is account loss.** If the user loses the device *and* the password manager sync
with no backup, there is no recovery — no "forgot password," no email reset. This is the big one.
The industry answer to this is recovery codes or multiple credentials; I have none of that yet. I
accepted it because there are zero users, and because the alternative — a recovery flow — is a
whole product I don't need to build for a solver's API keys. But it's a genuine product
limitation, and I'd want to solve it before this ever grew a real user base.

**The cut was hard, and it had to be.** The provider never reveals raw credentials or session
secrets in an exportable form — and crucially, *existing API keys weren't recoverable either.* A
phased migration where old keys keep working while new auth comes up was simply not possible.
There was no dual-run; there was a cutover. Zero users made that painless, and it's worth being
honest that the same decision at scale would look very different.

**Some of it was harder than the happy path suggests.** The library I use for the WebAuthn
ceremonies needed a pinned version to bundle correctly on the edge runtime. The local emulation
for my database was broken in the library I use, which forced a workaround in the test suite.
And I shipped — then caught in end-to-end tests, before it reached anyone — a client-side bug
where the sign-in response was wrapped one layer deeper than my UI expected. None of these are
reasons not to do it; they're the texture of doing auth yourself, and the e2e test that caught
the bug paid for itself on day one.

**What I kept:** sessions are HMAC-signed cookies (HttpOnly, SameSite=Strict, sliding
expiration); API keys are stored only as hashes; every ceremony is rate-limited per IP; there are
audit logs. For local development and CI, where the auth service isn't reachable, there's a
break-glass mode: a static key that bypasses the service so the API is testable without a
database or a sign-in. And because it's all mine, I can keep adding operator tools — like the
ban flag I shipped the same week: an operator flips a column and a user's login, sessions, and
every existing key stop working immediately, no rotation needed. Even the abuse story is
self-hosted now.

## Gained vs lost

What I gained: **independence from an identity vendor.** No OAuth matrix to maintain, no
provider's roadmap gating my features, no per-seat pricing looming over a free project. One-tap
identity that feels native to the platform. And the whole stack — sessions, keys, rate limits,
audit logs — runs on infrastructure I already had: the edge functions and the SQLite database
that Cloudflare gives me. The marginal cost of self-hosting auth was nearly zero because the
infrastructure was already there.

What I lost: a managed user list with a pretty admin UI. A recovery story I didn't have to
think about. Teams, roles, and orgs — which I don't need. And, honestly, the peace of mind of
*not* being responsible for auth, which is a real thing and the main reason most projects
shouldn't do what I did.

## When I'd still use a provider

This is the nuance that keeps me honest. If teeline had a team, billing, or compliance
requirements, I'd almost certainly still be on a hosted provider. Teams need shared accounts and
roles; monetization needs billing integration and fraud tooling; compliance needs audit
paperwork. Building those on top of passkeys is a serious project, and the hosted providers are
genuinely good at them.

What I built is right for a specific situation: a free, single-developer, API-first product with
zero users and no growth mandate. If you're in that situation, self-hosting auth is more
reachable than it looks — and passkeys are the reason. The ecosystem did the hard part; the
browser, the OS, and the password manager are doing the crypto for me. My job was just to store a
public key and not get in the way.

If you're building something similar and want the mechanics — the ceremony flow, the database
schema, the session cookie, the edge-runtime gotchas I hit — that's the recipe, and it's the
follow-up post. This one was the why.
