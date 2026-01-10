# Reading List: Diff Algorithms

Essential reading for understanding the algorithmic foundations of the array diffing optimizations in this codebase.

## Core Algorithm

### Myers Diff Algorithm
The foundational algorithm for computing minimal edit scripts between sequences.

- **Paper**: [An O(ND) Difference Algorithm and Its Variations](http://www.xmailserver.org/diff2.pdf) - Eugene W. Myers, 1986
  *The original paper. Sections 1-3 cover the core algorithm; Section 4 covers the linear-space refinement.*

- **Explanation**: [The Myers Diff Algorithm](https://blog.jcoglan.com/2017/02/12/the-myers-diff-algorithm-part-1/) - James Coglan
  *Excellent 3-part series with visualizations. Start here if the paper feels dense.*

## Histogram Diff

The algorithm we use (via imara-diff), which outperforms Myers for many real-world cases.

- **Blog**: [Histogram Diff](https://tiarkrompf.github.io/notes/?/diff-hierarchical/) - Tiark Rompf
  *Concise explanation of how histogram diff works and why it's faster.*

- **Source**: [Git's histogram diff implementation](https://github.com/git/git/blob/master/xdiff/xhistogram.c)
  *The reference implementation. Read the header comments for the algorithm description.*

- **Context**: [Git diff algorithm analysis](https://link.springer.com/article/10.1007/s10664-019-09772-z) - Nugroho et al., 2020
  *Academic comparison of diff algorithms in practice. Table 2 summarizes trade-offs.*

## Optimization Techniques

### Prefix/Suffix Stripping
Reduce problem size by skipping common elements at the start and end.

- **Discussion**: [Diff Strategies](https://neil.fraser.name/writing/diff/) - Neil Fraser
  *Practical guide covering prefix/suffix optimization, semantic cleanup, and performance. From the author of google-diff-match-patch.*

### Hash-Based Comparison
Using pre-computed hashes for fast equality checks.

- **Reference**: [Fowler-Noll-Vo Hash](http://www.isthe.com/chongo/tech/comp/fnv/)
  *FNV-1a is commonly used for this purpose. We use Rust's default hasher (SipHash) for security.*

## Quick Reference

| Phase | Technique | Key Insight |
|-------|-----------|-------------|
| 1 | Early termination | Skip diffing entirely for empty/identical inputs |
| 2 | Prefix/suffix matching | Common elements don't need diff computation |
| 3 | Pre-hashing | Hash once, compare many times |
| 5 | Histogram algorithm | Better than Myers for clustered changes |
