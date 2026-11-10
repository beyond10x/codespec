//! Strongly connected components without recursion, for the strata of a program and the `scc`
//! relation of the fact base.

/// The strongly connected components of the graph `edges` (node → successors), each node once,
/// a component after every component it reaches (Tarjan's order). An explicit stack stands for
/// the recursion, so a chain of any length is safe.
pub fn components(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    const NONE: usize = usize::MAX;
    let n = edges.len();
    let mut index = vec![NONE; n];
    let mut low = vec![0; n];
    let mut on = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut comps: Vec<Vec<usize>> = Vec::new();
    let mut next = 0;
    // (node, the position of the next successor to visit)
    let mut calls: Vec<(usize, usize)> = Vec::new();
    for root in 0..n {
        if index[root] != NONE {
            continue;
        }
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on[root] = true;
        calls.push((root, 0));
        while let Some(top) = calls.len().checked_sub(1) {
            let (v, k) = calls[top];
            if let Some(&w) = edges[v].get(k) {
                calls[top].1 += 1;
                if index[w] == NONE {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on[w] = true;
                    calls.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            calls.pop();
            if let Some(&(u, _)) = calls.last() {
                low[u] = low[u].min(low[v]);
            }
            if low[v] == index[v] {
                let mut comp = Vec::new();
                while let Some(w) = stack.pop() {
                    on[w] = false;
                    comp.push(w);
                    if w == v {
                        break;
                    }
                }
                comps.push(comp);
            }
        }
    }
    comps
}

#[cfg(test)]
mod tests {
    use super::components;

    #[test]
    fn components_come_after_what_they_reach_and_a_long_chain_is_safe() {
        // 0 -> 1 <-> 2 -> 3
        let c = components(&[vec![1], vec![2], vec![1, 3], vec![]]);
        let mut sorted: Vec<Vec<usize>> = c
            .iter()
            .map(|x| {
                let mut x = x.clone();
                x.sort();
                x
            })
            .collect();
        assert_eq!(sorted, vec![vec![3], vec![1, 2], vec![0]]);
        sorted.clear();
        let n = 1_000_000;
        let chain: Vec<Vec<usize>> = (0..n)
            .map(|k| if k + 1 < n { vec![k + 1] } else { vec![] })
            .collect();
        let c = components(&chain);
        assert_eq!(c.len(), n);
        assert_eq!(c[0], vec![n - 1]);
    }
}
