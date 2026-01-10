# Reading List: Diff Algorithms

Essential reading for understanding the algorithmic foundations of the array diffing optimizations in this codebase.

## Core Algorithm

### Myers Diff Algorithm

The foundational O(ND) algorithm for computing minimal edit scripts between sequences.

- **Paper**: [An O(ND) Difference Algorithm and Its Variations](http://www.xmailserver.org/diff2.pdf) - Eugene W. Myers, 1986
  *The original paper. Sections 1-3 cover the core algorithm; Section 4 covers the linear-space refinement.*

- **Interactive Visualization**: [Myers Diff Algorithm - Code & Interactive Visualization](https://blog.robertelder.org/diff-algorithm/) - Robert Elder
  *Best visual explanation. Lets you step through the algorithm on custom inputs.*

- **Tutorial**: [The Myers Diff Algorithm Part 1](https://blog.jcoglan.com/2017/02/12/the-myers-diff-algorithm-part-1/) - James Coglan
  *3-part series walking through the algorithm with clear diagrams. Start here if the paper feels dense.*

## Patience Diff

Foundation for the histogram algorithm. Uses unique lines as anchors.

- **Original Post**: [Patience Diff Advantages](https://bramcohen.livejournal.com/73318.html) - Bram Cohen
  *The inventor explains why patience diff produces more readable output than Myers.*

- **Summary**: [Patience Diff, a brief summary](https://alfedenzo.livejournal.com/170301.html) - Alfred
  *Concise explanation of the 4-step algorithm with examples.*

## Histogram Diff

The algorithm we use (via imara-diff). Extends patience diff with occurrence counting.

- **How It Works**: [How "histogram diff" actually works](https://www.raygard.net/2025/01/28/how-histogram-diff-works/) - Ray Gardner, 2025
  *Detailed walkthrough with pseudocode. The clearest explanation available.*

- **Implementation**: [A histogram diff implementation](https://www.raygard.net/2025/01/29/a-histogram-diff-implementation/) - Ray Gardner, 2025
  *Companion post with working C99 code and algorithm notes.*

- **Rust Library**: [imara-diff announcement](https://users.rust-lang.org/t/announcing-imara-diff-a-reliably-performant-diffing-library-for-rust/83276) - Pascal Kuthe
  *The library we use. Explains performance characteristics and when histogram beats Myers.*

- **Research**: [How Different Are Different diff Algorithms in Git?](https://link.springer.com/article/10.1007/s10664-019-09772-z) - Nugroho et al., 2020
  *Academic comparison recommending histogram for code changes. Table 2 summarizes trade-offs.*

## Optimization Techniques

### Prefix/Suffix Stripping

- **Guide**: [Diff Strategies](https://neil.fraser.name/writing/diff/) - Neil Fraser
  *Practical guide covering prefix/suffix optimization, semantic cleanup, and the half-match heuristic. From the author of google-diff-match-patch.*

## Quick Reference

| Phase | Technique | Key Insight |
|-------|-----------|-------------|
| 1 | Early termination | Skip diffing entirely for empty/identical inputs |
| 2 | Prefix/suffix matching | Common elements don't need diff computation |
| 3 | Pre-hashing | Hash once, compare many times |
| 5 | Histogram algorithm | Better than Myers for clustered changes |

## Suggested Reading Order

1. **Neil Fraser's Diff Strategies** - Quick overview of optimization techniques
2. **James Coglan's Myers tutorial** - Understand the baseline algorithm
3. **Bram Cohen's Patience Diff post** - Why unique-line matching helps
4. **Ray Gardner's Histogram posts** - How histogram extends patience
