use crate::bytecode::used_ops;
use crate::ir::Proto;
use crate::rng::Rng;

const ALPHA: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

struct Names {
    used: std::collections::HashSet<String>,
}

impl Names {
    fn new() -> Self {
        Names {
            used: Default::default(),
        }
    }
    fn get(&mut self, rng: &mut Rng) -> String {
        loop {
            let len = 3 + rng.below(4);
            let mut s = String::new();
            for _ in 0..len {
                s.push(ALPHA[rng.below(ALPHA.len())] as char);
            }
            if !self.used.contains(&s) {
                self.used.insert(s.clone());
                return s;
            }
        }
    }
}

pub struct VmBuild {
    pub source: String,
    pub handlers: usize,
}

pub fn emit(root: &Proto, rng: &mut Rng, decoder: &str) -> VmBuild {
    let mut n = Names::new();

    let run = n.get(rng);
    let disp = n.get(rng);
    let code = n.get(rng);
    let consts = n.get(rng);
    let kids = n.get(rng);
    let frame = n.get(rng);
    let ptr = n.get(rng);

    // Build the opcode permutation explicitly: remap[op] is the number that
    // op appears as in the dispatch table.
    let mut remap = [0u8; 48];
    let mut numbers: Vec<u8> = Vec::new();
    for slot in 0..48usize {
        loop {
            let cand = (rng.next_u64() % 190 + 20) as u8;
            if !numbers.contains(&cand) {
                numbers.push(cand);
                remap[slot] = cand;
                break;
            }
        }
    }

    let proto_lua = crate::bytecode::proto_lua(root, &remap);
    let used = used_ops(root);

    let mut names: Vec<(u8, String)> = Vec::new();
    for byte in &used {
        names.push((*byte, n.get(rng)));
    }

    let mut out = String::new();
    out.push_str(&format!("local {run}\n"));
    for (byte, name) in &names {
        out.push_str(&body(*byte, name, &run));
    }

    let mut entries = names.clone();
    let mut i = entries.len();
    while i > 1 {
        i -= 1;
        let j = rng.below(i + 1);
        entries.swap(i, j);
    }
    let table: Vec<String> = entries
        .iter()
        .map(|(b, name)| format!("[{}]={}", remap[*b as usize], name))
        .collect();
    out.push_str(&format!("local {disp}={{{}}}\n", table.join(",")));

    out.push_str(&format!(
        "{run}=function(P,...)\nlocal {frame}={{s={{...}},n=select('#',...),l={{}}}}\nlocal {consts}=P[4]\nlocal {code}=P[5]\nlocal {kids}=P[6]\nlocal {ptr}=1\nlocal z=#{code}\nwhile {ptr}<=z do\nlocal q={code}[{ptr}]\nlocal g={disp}[q[1]]\nif not g then error(q[1],0) end\nlocal r=g(P,{consts},{kids},{frame},q[2],q[3])\nif r==nil then {ptr}={ptr}+1\nelseif r[1]==1 then {ptr}=r[2]\nelse return r[2] end\nend\nreturn {frame}.s[{frame}.n]\nend\nreturn {run}({dec}({proto}))\n",
        run = run,
        frame = frame,
        consts = consts,
        code = code,
        kids = kids,
        ptr = ptr,
        disp = disp,
        proto = proto_lua,
        dec = decoder,
    ));

    VmBuild {
        source: out,
        handlers: names.len(),
    }
}

fn body(byte: u8, name: &str, run: &str) -> String {
    let b = match byte {
        0 => "F.n=F.n+1 F.s[F.n]=C[a+1]".into(),
        1 => "F.n=F.n+1 F.s[F.n]=nil".into(),
        2 => "F.n=F.n+1 F.s[F.n]=true".into(),
        3 => "F.n=F.n+1 F.s[F.n]=false".into(),
        4 => "F.n=F.n+1 F.s[F.n]=F.l[a+1]".into(),
        5 => "F.l[a+1]=F.s[F.n] F.s[F.n]=nil F.n=F.n-1".into(),
        6 => "F.s[F.n]=nil F.n=F.n-1".into(),
        7 => "F.n=F.n+1 F.s[F.n]=F.s[F.n-1]".into(),
        8 => bin("x+y"),
        9 => bin("x-y"),
        10 => bin("x*y"),
        11 => bin("x/y"),
        12 => bin("x//y"),
        13 => bin("x%y"),
        14 => bin("x^y"),
        15 => "F.s[F.n]=-F.s[F.n]".into(),
        16 => "F.s[F.n]=not F.s[F.n]".into(),
        17 => "F.s[F.n]=#F.s[F.n]".into(),
        18 => "local y=F.s[F.n] local x=F.s[F.n-1] F.n=F.n-1 F.s[F.n]=tostring(x)..tostring(y)".into(),
        19 => bin("x==y"),
        20 => bin("x~=y"),
        21 => bin("x<y"),
        22 => bin("x<=y"),
        23 => bin("x>y"),
        24 => bin("x>=y"),
        25 => "return{1,a}".into(),
        26 => "local v=F.s[F.n] F.s[F.n]=nil F.n=F.n-1 if not v then return{1,a} end".into(),
        27 => "local v=F.s[F.n] F.s[F.n]=nil F.n=F.n-1 if v then return{1,a} end".into(),
        28 => "F.n=F.n+1 F.s[F.n]={}".into(),
        29 => "local v=F.s[F.n] F.s[F.n]=nil F.n=F.n-1 F.s[F.n][C[a+1]]=v".into(),
        30 => "F.s[F.n]=F.s[F.n][C[a+1]]".into(),
        31 => "local v=F.s[F.n] F.s[F.n]=nil F.n=F.n-1 local i=F.s[F.n] F.s[F.n]=nil F.n=F.n-1 F.s[F.n][i]=v".into(),
        32 => "local i=F.s[F.n] F.s[F.n]=nil F.n=F.n-1 F.s[F.n]=F.s[F.n][i]".into(),
        33 => call(),
        34 => method(),
        35 => "return{2,F.s[F.n]}".into(),
        36 => format!("F.n=F.n+1 F.s[F.n]=function(...) local c=K[a+1] local save=F.s local sn=F.n F.s={{...}} F.n=select('#',...) local o={run}(c,table.unpack(F.s,1,F.n)) F.s=save F.n=sn return o end"),
        37 => "F.l[a+4]=false".into(),
        38 => "local st=F.l[a+3] or 1 F.l[a+1]=F.l[a+1]+st local lim=F.l[a+2] if(st>0 and F.l[a+1]>lim)or(st<0 and F.l[a+1]<lim)then return nil end F.l[a+4]=F.l[a+1]".into(),
        39 => "return{1,b}".into(),
        41 => String::new(),
        42 => "F.n=F.n+1 F.s[F.n]=_ENV[C[a+1]]".into(),
        43 => "_ENV[C[a+1]]=F.s[F.n] F.s[F.n]=nil F.n=F.n-1".into(),
        44 => String::new(),
        45 => "F.s[F.n]=tostring(F.s[F.n])".into(),
        46 => "local y=F.s[F.n] F.n=F.n-1 if not F.s[F.n] then F.s[F.n]=y end".into(),
        47 => "local y=F.s[F.n] F.n=F.n-1 if F.s[F.n] then F.s[F.n]=y end".into(),
        _ => "error('op')".into(),
    };
    format!("local function {name}(P,C,K,F,a,b)\n{b}\nend\n", name = name, b = b)
}

fn bin(e: &str) -> String {
    format!("local y=F.s[F.n] local x=F.s[F.n-1] F.n=F.n-1 F.s[F.n]={e}")
}

fn call() -> String {
    "local n=b local base=F.n-n local fn=F.s[base] local t={} for i=1,n do t[i]=F.s[base+i] end F.n=base-1 local r=fn(table.unpack(t,1,n)) F.n=F.n+1 F.s[F.n]=r".into()
}

fn method() -> String {
    "local n=b local base=F.n-n local o=F.s[base] local t={} for i=1,n do t[i]=F.s[base+i] end F.n=base-1 local m=o==nil and nil or o[C[a+1]] local r=m(o,table.unpack(t,1,n)) F.n=F.n+1 F.s[F.n]=r".into()
}
