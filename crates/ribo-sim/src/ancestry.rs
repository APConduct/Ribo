pub mod node {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub struct Id(pub u32);

    impl Id {
        #[inline]
        pub fn get(self) -> usize {
            self.0 as usize
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub parent: Option<node::Id>,
    pub children: Vec<node::Id>,
    pub label: String,
    pub born: u64,
    pub died: Option<u64>,
    pub alive: bool,
    pub alive_count: u32,
}
impl Node {
    pub fn is_extinct(self) -> bool {
        self.alive_count == 0
    }
}

#[derive(Clone, Debug)]
pub struct Ancestry {
    nodes: Vec<Node>,
    root: node::Id,
    active_root: node::Id,
}

pub mod prune {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Error {
        NotADescendant,
        NotAlive,
        NoChange,
    }
}

impl Ancestry {
    pub fn new(label: impl Into<String>, tick: u64) -> Ancestry {
        let root = Node {
            parent: None,
            children: Vec::new(),
            label: label.into(),
            born: tick,
            died: None,
            alive: true,
            alive_count: 1,
        };
        Ancestry {
            nodes: vec![root],
            root: node::Id(0),
            active_root: node::Id(0),
        }
    }

    pub fn root(&self) -> node::Id {
        self.root
    }

    pub fn active_root(&self) -> node::Id {
        self.active_root
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: node::Id) -> &Node {
        &self.nodes[id.get()]
    }

    pub fn nodes_mut(&mut self, id: node::Id) -> &mut Node {
        &mut self.nodes[id.get()]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.nodes.iter_mut()
    }

    pub fn spawn(&mut self, parent: node::Id, label: impl Into<String>, tick: u64) -> node::Id {
        let id = node::Id(self.nodes.len() as u32);
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            label: label.into(),
            born: tick,
            died: None,
            alive: true,
            alive_count: 1,
        });
        self.nodes[parent.get()].children.push(id);
        let mut cursor = Some(parent);
        while let Some(id) = cursor {
            self.nodes[id.get()].alive_count += 1;
            cursor = self.nodes[id.get()].parent;
        }
        id
    }

    pub fn kill(&mut self, id: node::Id, tick: u64) {
        if !self.nodes[id.get()].alive {
            return;
        }
        self.nodes[id.get()].alive = false;
        self.nodes[id.get()].died = Some(tick);

        let mut cursor = Some(id);
        while let Some(id) = cursor {
            self.nodes[id.get()].alive_count -= 1;
            cursor = self.nodes[id.get()].parent;
        }
    }

    pub fn is_alive(&self, id: node::Id) -> bool {
        self.nodes[id.get()].alive
    }

    pub fn is_descendant(&self, id: node::Id, ancestor: node::Id) -> bool {
        let mut cursor = Some(id);
        while let Some(id) = cursor {
            if id == ancestor {
                return true;
            }
            cursor = self.nodes[id.get()].parent;
        }
        false
    }

    pub fn active_clade_extinct(&self) -> bool {
        self.nodes[self.active_root.get()].alive_count == 0
    }

    pub fn active_population_count(&self) -> u32 {
        self.nodes[self.active_root.get()].alive_count
    }

    pub fn prune_to(&mut self, target: node::Id) -> Result<Vec<node::Id>, prune::Error> {
        if target == self.active_root {
            return Err(prune::Error::NoChange);
        }
        if !self.is_descendant(target, self.active_root) {
            return Err(prune::Error::NotADescendant);
        }
        if self.active_clade_extinct() {
            return Err(prune::Error::NotAlive);
        }
        let kept: Vec<node::Id> = self.subtree(target);
        let mut ceded: Vec<node::Id> = Vec::new();
        for id in self.subtree(self.active_root) {
            if !kept.contains(&id) {
                ceded.push(id);
            }
        }
        self.active_root = target;
        Ok(ceded)
    }

    pub fn subtree(&self, root: node::Id) -> Vec<node::Id> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            out.push(id);
            for c in self.nodes[id.get()].children.iter() {
                stack.push(*c);
            }
        }
        out
    }

    pub fn living_in_active_clade(&self) -> Vec<node::Id> {
        self.subtree(self.active_root)
            .into_iter()
            .filter(|id| self.nodes[id.get()].alive)
            .collect()
    }

    pub fn hiers_of(&self, id: node::Id) -> Vec<node::Id> {
        self.subtree(id)
            .into_iter()
            .filter(|n| *n != id && self.nodes[n.get()].alive)
            .collect()
    }

    pub fn rivals(&self) -> Vec<node::Id> {
        let mine = self.subtree(self.active_root);
        self.nodes
            .iter()
            .enumerate()
            .filter(|(i, n)| n.alive && !mine.contains(&node::Id(*i as u32)))
            .map(|(i, _)| node::Id(i as u32))
            .collect()
    }

    pub fn depth_of(&self, id: node::Id) -> usize {
        let mut depth = 0;
        let mut c = self.nodes[id.get()].parent;
        while let Some(p) = c {
            depth += 1;
            c = self.nodes[p.get()].parent;
        }
        depth
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn alive_counts_propagate() {
        let mut a = Ancestry::new("origin", 0);
        let root = a.root();
        let x = a.spawn(root, "x", 1);
        let y = a.spawn(root, "y", 1);
        let x1 = a.spawn(x, "x1", 2);
        assert_eq!(a.node(root).alive_count, 4);
        assert_eq!(a.node(x).alive_count, 2);
        a.kill(x1, 3);
        assert_eq!(a.node(x).alive_count, 1);
        assert_eq!(a.node(root).alive_count, 3);
        a.kill(y, 3);
        assert_eq!(a.node(root).alive_count, 2);
    }
    #[test]
    fn kill_is_idempotent() {
        let mut a = Ancestry::new("origin", 0);
        let root = a.root();
        let x = a.spawn(root, "x", 1);
        a.kill(x, 2);
        a.kill(x, 3);
        assert_eq!(a.node(root).alive_count, 1);
    }
    #[test]
    fn pruning_narrows_the_clade_and_cedes_the_rest() {
        let mut a = Ancestry::new("origin", 0);
        let root = a.root();
        let x = a.spawn(root, "x", 1);
        let y = a.spawn(root, "y", 1);
        let ceded = a.prune_to(x).unwrap();
        assert!(ceded.contains(&y));
        assert!(ceded.contains(&root));
        assert_eq!(a.active_root(), x);
        // The ceded branch is still alive in the world, just not yours.
        assert!(a.is_alive(y));
        assert!(a.rivals().contains(&y));
    }
    #[test]
    fn pruning_sideways_is_rejected() {
        // Pruning sideways is not allowed: you must prune to a descendant.
        let mut a = Ancestry::new("origin", 0);
        let root = a.root();
        let x = a.spawn(root, "x", 1);
        let y = a.spawn(root, "y", 1);
        // Narrow into `x`, which cedes `y`. `y` is now a sibling branch, not ours.
        a.prune_to(x).unwrap();
        assert_eq!(a.prune_to(y), Err(prune::Error::NotADescendant));
        assert_eq!(a.active_root(), x);
    }
    #[test]
    fn individual_death_and_clade_extinction_are_one_check() {
        let mut a = Ancestry::new("origin", 0);
        let root = a.root();
        let x = a.spawn(root, "x", 1);
        let _y = a.spawn(root, "y", 1);
        a.prune_to(x).unwrap();
        assert!(!a.active_clade_extinct());
        // The individual dies with no heirs. Run over, even though `y` lives on
        // as a rival: you traded that branch away when you transferred.
        a.kill(x, 5);
        assert!(a.active_clade_extinct());
    }
}
