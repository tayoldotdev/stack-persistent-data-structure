use std::io;
use std::io::Write;
use std::fs::File;
use std::process::Command;

#[derive(Debug, Clone)]
enum DataType {
    Int,
    Ptr,
    Bool,
}

type FrameIndex = usize;

struct Frame {
    data_type: DataType,
    previous: Option<FrameIndex>,
}

#[derive(Default)]
struct FrameAtor {
    frames: Vec<(Frame, usize)>,
    free: Vec<FrameIndex>,
}

impl FrameAtor {
    fn alloc(&mut self, init: Frame) -> FrameIndex {
        if let Some(result) = self.free.pop() {
            self.frames[result] = (init, 1);
            result
        } else {
            let result = self.frames.len();
            self.frames.push((init, 1));
            result
        }
    }

    fn acquire(&mut self, index: usize) {
        self.frames[index].1 += 1
    }

    fn release(&mut self, index: usize) {
        self.frames[index].1 -= 1;
        if self.frames[index].1 == 0 {
            if let Some(prev_index) = self.frames[index].0.previous {
                self.release(prev_index);
            }
            self.free.push(index);
        }
    }

    fn deref(&mut self, index: FrameIndex) -> Option<&Frame> {
        self.frames.get(index).map(|x| &x.0)
    }

    fn deref_mut(&mut self, index: FrameIndex) -> Option<&mut Frame> {
        self.frames.get_mut(index).map(|x| &mut x.0)
    }

    fn dump_dot(&self, mut sink: impl Write) -> io::Result<()> {
        writeln!(sink, "digraph Stacks {{")?;
        for (index, (frame, ref_count)) in self.frames.iter().enumerate() {
            if !self.free.contains(&index) {
                writeln!(sink, "    Node_{} [label=\"{:?} ({})\"]", index, frame.data_type, ref_count)?;
                if let Some(prev_index) = frame.previous {
                    writeln!(sink, "    Node_{} -> Node_{}", index, prev_index)?;
                }
            }
        }
        writeln!(sink, "}}")?;
        Ok(())
    }
}

#[derive(Default)]
struct TypeStack {
    top: Option<FrameIndex>
}

impl TypeStack {
    fn clone(&self, ator: &mut FrameAtor) -> Self {
        if let Some(top_index) = self.top {
            ator.acquire(top_index);
        }
        Self{ top: self.top }
    }

    fn push(&mut self, ator: &mut FrameAtor, data_type: DataType) {
        self.top = Some(ator.alloc(Frame{
            data_type,
            previous: self.top,
        }))
    }

    fn pop(&mut self, ator: &mut FrameAtor) {
        if let Some(top_index) = self.top {
            let prev = ator.deref_mut(top_index).unwrap().previous;
            if let Some(prev_index) = prev {
                ator.acquire(prev_index);
            }
            ator.release(top_index);
            self.top = prev;
        }
    }

    fn dump(&self, ator: &mut FrameAtor) {
        let mut top = self.top;
        while let Some(index) = top {
            let frame = ator.deref(index).unwrap();
            println!("[{:?}]", frame.data_type);
            top = frame.previous;
        }
    }
}

const RAND_A: u64 = 6364136223846793005;
const RAND_C: u64 = 1442695040888963407;

struct Rand {
    seed: u64,
}

impl Rand {
    fn rand(&mut self) -> u32 {
        self.seed = RAND_A.wrapping_mul(self.seed).wrapping_add(RAND_C);
        (self.seed >> 32) as u32
    }
}

fn rand_type(rand: &mut Rand) -> DataType {
    match rand.rand() % 3 {
        0 => DataType::Int,
        1 => DataType::Ptr,
        2 => DataType::Bool,
        _ => unreachable!(),
    }
}

fn generate_tree(ator: &mut FrameAtor, rand: &mut Rand, stack: &mut TypeStack, level: usize) {
    if level == 0 {
        return;
    }

    for _ in 0..3 {
        stack.push(ator, rand_type(rand));
    }

    let mut stack0 = stack.clone(ator);
    let mut stack1 = stack.clone(ator);
    generate_tree(ator, rand, stack, level-1);
    generate_tree(ator, rand,&mut stack0, level-1);
    generate_tree(ator, rand,&mut stack1, level-1);
}

fn main() {
    let mut rand = Rand{seed: 69};
    let mut ator = FrameAtor::default();
    let mut stack1 = TypeStack::default();
    generate_tree(&mut ator, &mut rand, &mut stack1, 4);

    let out_filepath = "out.dot";
    println!("[INFO] Generating `{}`", out_filepath);
    ator.dump_dot(File::create(out_filepath).unwrap()).unwrap();

    Command::new("dot")
        .args(["-Tsvg", "-O", out_filepath])
        .output() 
        .expect("dot command should've executed successfuly but NO");
}
