#import "@local/wrap-it:0.1.1": wrap-content
#import "overlay.typ": inset, overlay, page-size
#let wrap = wrap-content.with(align: right)

#set page(
  width: page-size.width,
  height: page-size.height,
  fill: rgb("#333333"),
  margin: inset,
  columns: 2,
)
#set text(size: 12.5pt, fill: white)
#set columns(gutter: 40pt)

//TODO: add an artificially aged photo of myself at age 25/30

= vision
// Target
target Valeriy Sakharov is a trader worth 25M\$

// ## Symptoms


#wrap(
  image("./assets/sbf_desk.png", width: 40%),
)[
casually swings deci-million positions; but very careful about the risk-management. Prefers to pyramid in when right; risking only existing profit whenever possible. Able to size the trades quickly and precisely, due to weekly playing poker at a high level.

has automatic rm systems, removing the need to worry about psychology. Doesn't have to worry about slippage, as algorthims are in-place to regulate that. His quant hedge-fund Liquid Alpha employes 6 people (>50% are math PhD); with practically 0 churn, as #link("https://www.instagram.com/p/DRsoKzhETqG/")[he ensures that every employee can achieve his goals within the organization]. Is #link("https://youtu.be/KZSXFPt4Vqk")[mindful of entropy], - hiring only prospects above average level on the team.
]

// ## Causes
#wrap(
  image("./assets/MartinSchmid.jpg", width: 30%),
)[
operating at the highest level means *making good decisions*. So he's world-class at building accurate predictions; updates them fast and accurately when new relevant information comes out; which he systematically tracks for outstanding positions. PhD-level at math; has made academic contributions on ideas introduced in Martin Schmid's papers, and on general RM solutions. Loves reading scientific papers. Embrasses #link("https://youtu.be/YC4T3g7QP5w?si=QkNuh2Sb1KKh0l-Q")[contradiction].
]

he is ruthlessly pain-seeking, only pursuing delayed gratification. Deep work is the most rewarding experience for him, - religious #link("https://youtu.be/3uXYZwS_cck")[12h worker]. Does focus-meditation to train ability to deep-work in isolation. Knows how to #link("/home/v/s/g/notes/action_sets/pre_work_env_with_easy_distractions.md")[snap himself into work] on demand. Optimizes environment to minimize necessary decision making to stupid extent /*TODO: add yt vid link to Sam or Charlie*/.

is good at prioritization, doesn't get tempted into starting with tasks that look easy. Pays close attention to distinguishing #link("https://youtu.be/sslfgLMOyWY?si=HzFsYqDTn4o9pAHf")[which activities are "weeds", and tearing them out]. #link("https://www.youtube.com/watch?v=CCtKq06wuH8&t=420s")[Obsessed with finishing things]. Obsessed with speed, - has maniacal sence of urgency; disregards all arbitrary deadlines and pulls future forward. Plans through use of #link("https://youtu.be/N8aG6Nu3d9w?si=TEqfPthHdqG3vwh7")[cloning method].

forever cogniscent of the need to pay down ignorance debt/*literal cost of you not knowing how to make millions right now is the difference between where you are and where you could be*/. Greedily #link("https://youtu.be/6BQ3whjWG3M?t=24m23s")[collects compacted expertise] from absolutely everyone ahead of me in my field on any axis; readily overpaying multiples of favor-units to get in return application of their experience, Hormozi-style. Conscious of the #link("https://www.youtube.com/watch?v=UUiMaSbr79w")[identity cycle] and knows how to detect being at the upper boundary; pull force of the undesirable identity; tools to overpower it.

delegates ruthlessly; being aware of #link("https://youtu.be/xm2cA5Y5Ru4")[comparative activity hourly rates]. Treats his company like a war, enjoys biographies of great conquerers, #link("https://youtu.be/7Y7Yxf67g7Q?si=2b9sJ4H8QHxqar6y")[like Musk and Napoleon has organization split into Core Units and fights on the frontlines]. And once a direction is planned and chosen, there is no deliberation, - I switch to 100% execution, 0 deliberation; #link("https://youtu.be/LNdsl52emwQ?t=202")[like Rockerfeller did].

never sacrifices reputation, ruthless about defending it.

== tangible
#let tangible-pic(caption, path) = [
  #box(width: 130pt, caption)
  #image(path, height: 80pt)
]
#grid(
  columns: 3,
  gutter: 15pt,
  align: bottom,
  tangible-pic([- drives a Mercedes S-class], "./assets/Mercedez_S-class_Berline.png"),
  tangible-pic([- lives in #link("https://moscowestates.com/property/apartment-110-sqm-on-the-56th-floor-in-the-federation-tower/")[Moscow City]], "./assets/apt_main.jpg"),
  tangible-pic([- owns a house on a ski slope (\~1.4\$M) in Switzerland.], "./assets/ski_slope_house.jpg"),
)
  Goes there for 1-2 weeks yearly; planning the life for the next year, in semi-seclusion
- owns a 10k Rolex, but prefers wearing Casio over it
- doesn't have a wake-up alarm
  // I think it's important to drive this as one of the invariants, as [you can't self-diagnose sleep debt](https://x.com/aakashgupta/status/2042720283447247121) /*[The Cumulative Cost of additional Wakefulness](https://academic.oup.com/sleep/article-abstract/26/2/117/2709164?redirectedFrom=fulltext&login=false#no-access-message)*/

== fitness
can do all of:
- planche
- front-lever
- 1+ one-arm pull-up on each hand
- hand-stand pushups

// additional indicators of fitness could be: {visible abs, bicep vein, etc}, but I'm pretty sure the perf-based metrics above already force all the cosmetics in order; while also being actionable.

== family
#wrap(
  image("./assets/Priscilia.png", width: 32%),
)[
married to someone who increases his productive output /*(likely Priscilia)*/. No kids yet, but is in a position to easily provide for them if needed.

Besides being in such arrangement, keeps strict rules about what food and other temptations can physically be in the house (or in reach). Is very careful about reinforcement rules for himself and everyone he often comes in contact with.
]

#wrap(
  image("./assets/AndreySakharov_restored.png", width: 40%),
)[
Continues Andrey's legacy. With very stable profits on 25M\$ of capital; it is on track to continue growing exponentially.
]

= other
== optimize for
a single metric to tie-brake decisions:
_*drawdown protection*_

== target beliefs
/*
section where I list beliefs that need to be updated.

to compile, - list all the beliefs/opinions/feelings that have prevented you from having had achieved the goal already. 
*/
for each of these, imagine a picture where the statement is true, and you have a good feeling in relation to it.

- math is one of the most fun things I can do, beating any other leasurely activity
- excruciatingly analyzing and recording my trades, then comparing tiniest of similarities on history, is one of the most rewarding things to do.
- I love managing people
- as a business owner, I love that problems, that end up on my table, are the most difficult ones that business has to offer

== habits to establish
- note down distractions (that successfully diverted your attention away from work)
- prioritize hardest task in todo lists // currently don't do any prioritization once a list is constructed, - just going off of what feels right. And what feels right is what is easy atm. So I end up working on often much less relevant stuff.
- reinforce horrid fear of not succeeding. You must have a fully formed picture of exactly where you will be; same as what you are running from. I would rather die than become like those pathetic loosers.
- $>= 10%$ monthly earnings _*must*_ be spent on education. In the beginning that will be paying people directly; later on more and more just the cost of running expirements.
- every inconducive emotion preventing me from making a necessary action, triggers #link("https://youtu.be/33zWDyEHqM4?t=3041&si=2mZl_xPyRakC6zll")[DISARM protocol], and then gets logged to the journal postfactum.

#v(40pt)
#overlay()
