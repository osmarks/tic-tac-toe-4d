use fixedbitset::FixedBitSet;
use lines::LINE_BITSETS;
use web_sys::{Document, MessageEvent};
use yew::prelude::*;
use std::collections::HashMap;
use std::cmp;
use std::iter::FromIterator;
use bincode::{config, Decode, Encode};
use wasm_bindgen::prelude::*;

mod lines;

// coordinates: cell layer row column
#[derive(PartialEq, PartialOrd, Eq, Hash, Ord, Debug, Clone, Copy)]
struct Coord(u8);
impl Coord {
    fn from_indices(c: u8, l: u8, r: u8, o: u8) -> Self {
        Coord((c << 6) + (l << 4) + (r << 2) + o)
    }
    fn unpack(&self) -> (u8, u8, u8, u8) {
        (self.0 >> 6 & 3, self.0 >> 4 & 3, self.0 >> 2 & 3, self.0 & 3)
    }
    fn rotate(&self, by: u8) -> Self {
        Coord(self.0.rotate_right((by * 2).into()))
    }
}

#[derive(PartialEq, PartialOrd, Eq, Hash, Debug, Clone, Copy, Encode, Decode)]
struct Player(u8);

impl Player {
    const EMPTY: Self = Player(0);

    fn opponent(self) -> Self { Player(3 - self.0) }
    fn index(self) -> usize { self.0 as usize - 1 }
}

const PLAYER_COUNT: usize = 2;
const SIZE: usize = 4;
const SIZE_U8: u8 = SIZE as u8;
const GRID_SIZE: usize = SIZE * SIZE * SIZE * SIZE;

type LineContents = [u8; PLAYER_COUNT];

#[derive(Clone, Encode, Decode)]
struct Board {
    slots: [Player; GRID_SIZE],
    line_occupation: HashMap<usize, LineContents>,
    winner: Option<Player>
}

impl Board {
    fn new() -> Self {
        Board {
            slots: [Player::EMPTY; GRID_SIZE],
            line_occupation: HashMap::new(),
            winner: None
        }
    }
    #[inline]
    fn get(&self, c: Coord) -> Player {
        self.slots[c.0 as usize]
    }
    fn set(&mut self, c: Coord, playing: Player) {
        self.slots[c.0 as usize] = playing;
        for in_line in lines::LINE_MAP[c.0 as usize].iter() {
            let contents = self.line_occupation.entry(*in_line).or_insert([0; 2]);
            contents[playing.index()] += 1;
            if contents[playing.index()] == SIZE_U8 {
                self.winner = Some(playing)
            }
        }
    }
    #[inline]
    // danger to us, danger from us
    fn line_danger(us: Player, line: &LineContents) -> (u8, u8) {
        let them = us.opponent();
        if line[them.index()] > 0 && line[us.index()] > 0 { (0, 0) }
        else if line[them.index()] > 0 {
            (line[them.index()], 0)
        }
        else if line[us.index()] > 0 {
            (0, line[us.index()])
        } else {
            unreachable!()
        }
    }
    fn board_danger(&self, us: Player) -> (u8, u8) {
        let mut our_danger = 0;
        let mut their_danger = 0;
        for line in self.line_occupation.values() {
            let (our, their) = Board::line_danger(us, line);
            our_danger = cmp::max(our, our_danger);
            their_danger = cmp::max(their, their_danger);
        }
        (our_danger, their_danger)
    }
    fn good_moves(&self, us: Player) -> FixedBitSet {
        let (our_danger, their_danger) = self.board_danger(us);
        let mut slots = FixedBitSet::with_capacity(GRID_SIZE);
        let unoccupied = FixedBitSet::from_iter(self.positions_owned_by(Player::EMPTY));
        
        // TODO defactor, refactor, etc
        if our_danger < their_danger { // attack
            for (line_id, line) in self.line_occupation.iter() {
                if Board::line_danger(us, line).1 == their_danger {
                    slots.union_with(&LINE_BITSETS[*line_id]);
                }
            }
        } else if our_danger > their_danger { // defend
            for (line_id, line) in self.line_occupation.iter() {
                if Board::line_danger(us, line).0 == our_danger {
                    slots.union_with(&LINE_BITSETS[*line_id]);
                }
            }
        } else { // neutral I guess
            return unoccupied;
        }
        slots.intersect_with(&unoccupied);
        slots
    }
    fn positions_owned_by(&self, player: Player) -> impl Iterator<Item=usize> + '_ {
        self.slots.iter().enumerate().filter(move |(_, s)| **s == player).map(|(id, _)| id)
    }
}

fn negamax(board: Board, depth: u8, mut alpha: i8, beta: i8, player: Player, m: i8) -> i8 {
    match board.winner {
        Some(id) if id == player => return m * 64,
        Some(_) => return m * -64,
        _ => ()
    };
    if depth == 0 {
        let (our_danger, their_danger) = board.board_danger(player);
        return m * ((their_danger as i8) - (our_danger as i8));
    }
    let mut value = -127;
    for possible_move in board.good_moves(player).ones() {
        let mut new = board.clone();
        new.set(Coord(possible_move as u8), player);
        value = cmp::max(value, -negamax(new, depth - 1, -beta, -alpha, player.opponent(), -m));
        alpha = cmp::max(alpha, value);
        if alpha >= beta { break }
    }
    value
}

fn minimax_policy(board: &Board, player: Player, smarter: bool) -> u8 {
    let (mut best, mut best_value) = (0, -128);
    for possible_move in board.good_moves(player).ones() {
        let possible_move = possible_move as u8;
        let mut new_board = board.clone();
        new_board.set(Coord(possible_move), player);
        let score = negamax(new_board, if smarter { 3 } else { 2 }, -127, 127, player.opponent(), -1);
        if score > best_value {
            best = possible_move;
            best_value = score;
        }
    }
    best
}

#[wasm_bindgen]
pub fn run_ai(state: Vec<u8>) -> Vec<u8> {
    let config = config::standard();
    let decoded: (Board, Player, bool) = bincode::decode_from_slice(state.as_slice(), config).unwrap().0;
    let result = minimax_policy(&decoded.0, decoded.1, decoded.2);
    bincode::encode_to_vec(result, config).unwrap()
}

#[wasm_bindgen]
extern "C" {
    fn run_ai_background(state: Vec<u8>);
    fn win_callback(player: u8);
}

fn dispatch_ai_run(board: &Board, player: Player, smarter: bool) {
    run_ai_background(bincode::encode_to_vec((board, player, smarter), config::standard()).unwrap());
}

enum Msg {
    Click(Coord),
    SetHiglight(Coord),
    Unhighlight,
    Rotate,
    SetTileIdentity(bool),
    SetLineAssist(bool),
    AIRunDone(Vec<u8>),
    SetSlightlySmarterAI(bool)
}

struct Model {
    board: Board,
    highlights: [u8; GRID_SIZE],
    current_player: Player,
    rotation: u8,
    enable_tile_identity: bool,
    enable_line_assist: bool,
    waiting_for_ai: bool,
    slightly_smarter_ai: bool
}

fn rescale(x: u8, scale: f32, min: u8, max: u8) -> f32 {
    let xf = x as f32;
    let minf = min as f32;
    let maxf = max as f32;
    ((xf - minf) / (maxf - minf)) * scale
}

fn indicator_bg(coord: Coord) -> String {
    let (c, l, r, o) = coord.unpack();
    format!("background-color: oklab(80% {} {} / 40%); background-image: url(assets/horiz{}.svg), url(assets/vert{}.svg", rescale(c, 0.5, 0, 3) - 0.25, rescale(l, 0.5, 0, 3) - 0.25, r, o)
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            board: Board::new(),
            current_player: Player(1),
            highlights: [0; GRID_SIZE],
            rotation: 0,
            enable_line_assist: false,
            enable_tile_identity: false,
            waiting_for_ai: false,
            slightly_smarter_ai: false
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Click(pos) => {
                if self.board.winner.is_some() {
                    self.board = Board::new();
                    self.current_player = Player(1);
                    return true;
                }
                if self.current_player != Player(1) {
                    return true;
                }
                if self.board.get(pos) == Player::EMPTY {
                    self.board.set(pos, self.current_player);
                    self.switch_player();
                }
                true
            },
            Msg::SetHiglight(pos) => {
                self.set_highlights(pos.0);
                true
            },
            Msg::Unhighlight => {
                self.highlights = [0; GRID_SIZE];
                true
            },
            Msg::Rotate => {
                self.rotation += 1;
                self.rotation %= 4;
                true
            },
            Msg::SetLineAssist(x) => {
                self.enable_line_assist = x;
                true
            },
            Msg::SetTileIdentity(x) => {
                self.enable_tile_identity = x;
                true
            },
            Msg::AIRunDone(pos) => {
                self.board.set(Coord(bincode::decode_from_slice(&pos, config::standard()).unwrap().0), self.current_player);
                self.switch_player();
                self.waiting_for_ai = false;
                true
            },
            Msg::SetSlightlySmarterAI(x) => {
                self.slightly_smarter_ai = x;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let line_assist = self.enable_line_assist;
        let tile_identity = self.enable_tile_identity;
        let slightly_smarter_ai = self.slightly_smarter_ai;
        html! {
            <div>
                <div class="board">
                    { for (0..SIZE_U8).map(|c| html! {
                        <div class="cell" key={c}>
                        { for (0..SIZE_U8).map(|l| html! {
                            <div class="layer" key={l}>
                            { for (0..SIZE_U8).map(|r| html! {
                                <div class="row" key={r}>
                                { for (0..SIZE_U8).map(|o| {
                                    let id = Coord::from_indices(c, l, r, o).rotate(self.rotation);
                                    let hl = self.highlights[id.0 as usize];
                                    html! {
                                        <div
                                            class={format!("slot slot-{}", self.board.get(id).0)}
                                            onclick={ctx.link().callback(move |_| Msg::Click(id))}
                                            onmouseover={ctx.link().callback(move |_| Msg::SetHiglight(id))}
                                            onmouseout={ctx.link().callback(move |_| Msg::Unhighlight)}
                                            key={o}>
                                            <div style={if tile_identity && self.board.get(id) == Player::EMPTY { indicator_bg(id) } else { String::new() }} class="abspos-indicator">
                                                { if hl != 0 && line_assist { html! {
                                                    <div style={format!("background-color: hsl({}deg, 100%, 60%)", 30 + 20 * (hl as usize))} class="highlight"></div>
                                                } } else { html! {} } }
                                            </div>
                                        </div>
                                    }
                                }) }
                                </div>
                            }) }
                            </div>
                        }) }
                        </div>
                    }) }
                </div>
                <div>
                    { match self.board.winner {
                        Some(w) => format!("{} wins. Click to reset.", w.0),
                        None => format!("Player {}", self.current_player.0)
                    } }
                </div>
                <div>{ if self.waiting_for_ai { "Waiting for AI..." } else { "" } }</div>
                <div>
                        <button onclick={ctx.link().callback(move |_| Msg::Rotate)} class="control">{ "Rotate" }</button>
                        <span class="control">
                            <input id="lineassist" name="lineassist" type="checkbox" checked={self.enable_line_assist}
                                oninput={ctx.link().callback(move |_| Msg::SetLineAssist(!line_assist))} />
                            <label for="lineassist">{"Line Assist"}</label>
                        </span>
                        <span class="control">
                            <input id="tileidentity" name="tileidentity" type="checkbox" checked={self.enable_tile_identity}
                                oninput={ctx.link().callback(move |_| Msg::SetTileIdentity(!tile_identity))} />
                            <label for="tileidentity">{"Tile Identity"}</label>
                        </span>
                        <span class="control">
                            <input id="slightlysmarterai" name="slightlysmarterai" type="checkbox" checked={self.slightly_smarter_ai}
                                oninput={ctx.link().callback(move |_| Msg::SetSlightlySmarterAI(!slightly_smarter_ai))} />
                            <label for="slightlysmarterai">{"Slightly Smarter AI"}</label>
                        </span>
                </div>
            </div>
        }
    }
}

impl Model {
    fn switch_player(&mut self) {
        if let Some(Player(winner)) = self.board.winner {
            if winner == self.current_player.0 {
                win_callback(winner);
            }
        }
        self.current_player = self.current_player.opponent();
        if self.current_player.0 == 2 {
            self.waiting_for_ai = true;
            dispatch_ai_run(&self.board, self.current_player, self.slightly_smarter_ai)
        }
    }

    fn set_highlights(&mut self, slot: u8) {
        self.highlights = [0; GRID_SIZE];
        for (i, line_id) in lines::LINE_MAP[slot as usize].iter().enumerate() {
            for pos in lines::LINES[*line_id].iter() {
                self.highlights[*pos as usize] = (i + 1) as u8;
            }
        }
    }
}

#[wasm_bindgen]
pub fn main() -> JsValue {
    wasm_logger::init(wasm_logger::Config::default());
    let app = yew::start_app_in_element::<Model>(web_sys::window().unwrap().document().unwrap().get_element_by_id("app").unwrap());
    let fnbox = Box::new(move |x| app.send_message(Msg::AIRunDone(x)));
    let closure: Closure<dyn FnMut(Vec<u8>)> = Closure::wrap(fnbox);
    closure.into_js_value()
}