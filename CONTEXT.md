# Domain Glossary

Single-creator membership site. One Creator publishes content; Fans pay for access.

| Term | Definition |
|---|---|
| **Creator** | The single owner of the site. Exactly one user holds the `creator` role; everyone else is a Fan. Publishes content, defines Tiers, sets prices. |
| **Fan** | Any registered non-creator user. May hold a Subscription and/or Purchases. Anonymous visitors are not Fans. |
| **Tier** | A named, ordered membership level (e.g. Bronze < Silver < Gold, ordered by `level`). Higher tiers always include everything lower tiers can access. |
| **Subscription** | A Fan's active recurring membership at exactly one Tier. At most one active Subscription per Fan. |
| **Purchase** | A one-time buy of a single Post or a whole Series. Permanent: does not expire when a Subscription lapses. |
| **Entitlement** | The decision of whether a given user (or anonymous visitor) may access a given Post. Computed, never stored: free/rolled-free OR subscription at ≥ the post's minimum tier OR purchase of the post OR purchase of its series. |
| **Post** | The single unit of published content. Has exactly one *kind*: `update`, `article`, `audio`, `video`, or `images`. Always belongs to a Series (standalone updates live in the hidden Feed series). |
| **Series** | An ordered collection of Posts (a season, a course, a serialized book). Can be sold as a bundle: buying a Series entitles the buyer to all its current *and future* Posts. |
| **Feed series** | The hidden default Series that holds standalone `update` posts. Never shown as a Series in the UI. |
| **Gated** | A Post that requires an Entitlement beyond "anyone": it has a minimum tier and/or an individual price. |
| **Free** | A Post with no minimum tier and no price — visible to everyone including anonymous visitors. |
| **Rolling paywall / Rolled-free** | A Gated Post with a `free_at` time becomes Free once that time passes. The gate is scheduled to expire; the content is not re-gated afterwards. |
| **Livestream** *(reserved, v2)* | A future Entitlement subject: live video gated by the same rule as Posts. Not implemented; the Entitlement vocabulary deliberately does not assume its subject is a Post forever. |
