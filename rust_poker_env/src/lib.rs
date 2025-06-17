use pyo3::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use pyo3::types::{PyDict, PyTuple};
use pyo3::ToPyObject;
use rs_poker::core::{Hand, Rankable, Rank};
use std::cmp::Reverse;

#[derive(Debug, Clone, PartialEq)]
#[pyclass]
pub enum Action {
    #[pyo3(name = "FOLD")]
    Fold,
    #[pyo3(name = "CHECK")]
    Check,
    #[pyo3(name = "CALL")]
    Call,
    #[pyo3(name = "RAISE")]
    Raise,
}

impl ToPyObject for Action {
    fn to_object(&self, py: Python) -> PyObject {
        match self {
            Action::Fold => "fold".to_object(py),
            Action::Check => "check".to_object(py),
            Action::Call => "call".to_object(py),
            Action::Raise => "raise".to_object(py),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[pyclass]
pub enum Phase {
    #[pyo3(name = "PREFLOP")]
    Preflop,
    #[pyo3(name = "FLOP")]
    Flop,
    #[pyo3(name = "TURN")]
    Turn,
    #[pyo3(name = "RIVER")]
    River,
    #[pyo3(name = "SHOWDOWN")]
    Showdown,
}

impl ToPyObject for Phase {
    fn to_object(&self, py: Python) -> PyObject {
        match self {
            Phase::Preflop => "preflop".to_object(py),
            Phase::Flop => "flop".to_object(py),
            Phase::Turn => "turn".to_object(py),
            Phase::River => "river".to_object(py),
            Phase::Showdown => "showdown".to_object(py),
        }
    }
}

#[pyclass]
pub struct PokerEnv {
    #[pyo3(get, set)]
    agents: Vec<PyObject>,
    #[pyo3(get, set)]
    dead_agents: Vec<PyObject>,
    #[pyo3(get, set)]
    names: Vec<String>,
    #[pyo3(get, set)]
    dead_names: Vec<String>,
    #[pyo3(get)]
    num_players: usize,
    #[pyo3(get)]
    small_blind: i32,
    #[pyo3(get)]
    big_blind: i32,
    #[pyo3(get)]
    max_raise: i32,
    #[pyo3(get)]
    initial_stack: i32,
    #[pyo3(get, set)]
    stacks: Vec<i32>,
    #[pyo3(get, set)]
    dealer_pos: usize,
    #[pyo3(get, set)]
    bets: Vec<i32>,
    #[pyo3(get, set)]
    folded: Vec<bool>,
    #[pyo3(get, set)]
    all_in: Vec<bool>,
    #[pyo3(get, set)]
    rewards: Vec<i32>,
    #[pyo3(get, set)]
    current_phase: Phase,
    #[pyo3(get, set)]
    current_player: usize,
    #[pyo3(get, set)]
    deck: Vec<String>,
    #[pyo3(get, set)]
    player_cards: Vec<Vec<String>>,
    #[pyo3(get, set)]
    community_cards: Vec<String>,
}

#[pymethods]
impl PokerEnv {
    #[new]
    /// Init poker env
    pub fn new(
        _py: Python,
        agents: Vec<PyObject>,
        small_blind: i32,
        big_blind: i32,
        initial_stack: i32,
    ) -> PyResult<Self> {
        let num_players = agents.len();
        let mut poker_env = PokerEnv {
            agents: agents.clone(),
            dead_agents: Vec::new(),
            num_players: agents.len(),
            names: (0..num_players).map(|i| format!("player_{}", (b'A' + i as u8) as char)).collect(),
            dead_names: Vec::new(),
            small_blind,
            big_blind,
            max_raise: 0,
            initial_stack,
            stacks: vec![initial_stack; num_players],
            dealer_pos: 0,
            bets: vec![0; num_players],
            folded: vec![false; num_players],
            all_in: vec![false; num_players],
            rewards: vec![0; num_players],
            current_phase: Phase::Preflop,
            current_player: 0,
            deck: Vec::new(),
            player_cards: vec![Vec::new(); num_players],
            community_cards: Vec::new(),
        };

        poker_env.reset()?;
        Ok(poker_env)
    }

    /// Reset the env for a new round
    pub fn reset(&mut self) -> PyResult<()> {
        // Reset game state
        self.bets = vec![0; self.num_players];
        self.folded = vec![false; self.num_players];
        self.all_in = vec![false; self.num_players];
        self.rewards = vec![0; self.num_players];
        self.current_phase = Phase::Preflop;
        self.dealer_pos = (self.dealer_pos + 1) % self.num_players;
        self.current_player = (self.dealer_pos + 3) % self.num_players;

        // Create and shuffle deck
        let ranks = vec!["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"];
        let suits = vec!["h", "d", "c", "s"];
        self.deck = ranks
            .iter()
            .flat_map(|&rank| suits.iter().map(move |&suit| format!("{}{}", rank, suit)))
            .collect::<Vec<String>>();
        self.deck.shuffle(&mut thread_rng());

        // Distribute private cards
        self.player_cards = vec![Vec::new(); self.num_players];
        for i in 0..self.num_players {
            self.player_cards[i] = vec![
                self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty"))?,
                self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty"))?,
            ];
        }

        // Reset community cards
        self.community_cards = Vec::new();

        // Force blinds
        let sb_pos = (self.dealer_pos + 1) % self.num_players;
        let bb_pos = (self.dealer_pos + 2) % self.num_players;
        self.apply_bet(sb_pos, self.small_blind.min(self.stacks[sb_pos]))?;
        self.apply_bet(bb_pos, self.big_blind.min(self.stacks[bb_pos]))?;

        self.max_raise = self.bets.iter().max().copied().unwrap_or(0);

        Ok(())
    }

    /// Apply a bet for a player
    pub fn apply_bet(&mut self, player: usize, amount: i32) -> PyResult<()> {
        self.bets[player] = amount;
        if self.stacks[player] - self.bets[player] == 0 {
            self.all_in[player] = true;
        }
        Ok(())
    }

    /// Return all available actions for the current player
    pub fn get_available_actions(&mut self) -> PyResult<Vec<Py<PyTuple>>> {
        let mut actions: Vec<Py<PyTuple>> = Vec::new();
        let current_bet = self.bets[self.current_player];
        let current_stack = self.stacks[self.current_player];
        let max_bet = self.bets.iter().max().copied().unwrap_or(0);

        // No action if all in
        if self.all_in[self.current_player] {
            return Ok(actions);
        };

        // Always fold
        Python::with_gil(|py| {
            actions.push(PyTuple::new_bound(py, [Action::Fold.to_object(py)]).into());
        });

        let sum_all_in: usize = self.all_in.iter().map(|&b| b as usize).sum();
        let sum_folded: usize = self.folded.iter().map(|&b| b as usize).sum();

        if sum_all_in + sum_folded == self.folded.len() - 1 {
            if current_bet != max_bet {
                let call_amount = max_bet.min(current_stack);
                Python::with_gil(|py| {
                    actions.push(PyTuple::new_bound(py, [Action::Call.to_object(py), call_amount.to_object(py)]).into());
                });
            }
            return Ok(actions)
        };

        // "Check" is the bet of the player is equal to the max_bet, "Call" if not
        if current_bet == max_bet {
            Python::with_gil(|py| {
                actions.push(PyTuple::new_bound(py, [Action::Check.to_object(py)]).into());
            });
        } else {
            let call_amount = max_bet.min(current_stack);
            Python::with_gil(|py| {
                actions.push(PyTuple::new_bound(py, [Action::Call.to_object(py), call_amount.to_object(py)]).into());
            });
        };

        if current_stack > max_bet {
            let raise_range: (i32, i32);
            if current_stack >= max_bet*2 {
                raise_range = (max_bet + self.max_raise, current_stack);
            } else {
                raise_range = (current_stack, current_stack);
            }
            Python::with_gil(|py| {
                actions.push(PyTuple::new_bound(py, [Action::Raise.to_object(py), raise_range.to_object(py)]).into());
            });
        };

        Ok(actions)
    }

    /// Return observable state of game from the POV of the current player
    pub fn get_state(&mut self) -> PyResult<Py<PyDict>> {
        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item("player_cards", self.player_cards[self.current_player].clone())?;
            dict.set_item("community_cards", self.community_cards.clone())?;
            dict.set_item("stacks", self.stacks.clone())?;
            dict.set_item("bets", self.bets.clone())?;
            dict.set_item("phase", &self.current_phase)?;
            dict.set_item("current_player", self.current_player)?;
            dict.set_item("folded", self.folded.clone())?;
            dict.set_item("all_in", self.all_in.clone())?;
            Ok(dict.into())
        })
    }

    /// Print overall state
    pub fn overall_state(&mut self) -> PyResult<()> {
        println!("phase: {0:?}\nplayers_cards: {1:?}\ncommunity_cards: {2:?}\nfolded: {3:?}')\nall_in: {4:?}\nstacks: {5:?}\nbets: {6:?}\n",
                    self.current_phase,
                    self.player_cards,
                    self.community_cards,
                    self.folded,
                    self.all_in,
                    self.stacks,
                    self.bets);
        Ok(())
    }

    /// Proceed 1 turn of bet
    pub fn step_bid(&mut self, verbose: bool) -> PyResult<()> {
        let mut last_bet = (self.current_player + self.num_players - 1) % self.num_players;
        loop {
            if self.folded[self.current_player] {
                if last_bet == self.current_player {
                    break;
                }
                self.current_player = (self.current_player + 1) % self.num_players;
                continue;
            }

            let agent = self.agents[self.current_player].clone();
            let state = self.get_state()?;
            let available_actions = self.get_available_actions()?;

            if available_actions.len() == 1 {
                break;
            }

            if !available_actions.is_empty() {
                // Call agent's choose_action method
                let action = Python::with_gil(|py| {
                    agent.call_method1(py, "choose_action", (state, available_actions))
                })?;

                if verbose {
                    println!("{} has {}", self.names[self.current_player], action)
                }

                // Extract the first element of the action tuple
                let action_type = Python::with_gil(|py| {
                    action
                        .bind(py)
                        .get_item(0)?
                        .extract::<String>()
                })?;

                match action_type.as_str() {
                    "fold" => {
                        self.folded[self.current_player] = true;
                    }
                    "check" => {}
                    "call" => {
                        let amount = Python::with_gil(|py| {
                            action.bind(py).get_item(1)?.extract::<i32>()
                        })?;
                        self.apply_bet(self.current_player, amount)?;
                    }
                    "raise" => {
                        let amount = Python::with_gil(|py| {
                            action.bind(py).get_item(1)?.extract::<i32>()
                        })?;
                        let raise_amount = amount - self.bets.iter().max().copied().unwrap_or(0);
                        if raise_amount > self.max_raise {
                            self.max_raise = raise_amount;
                        }
                        self.apply_bet(self.current_player, amount)?;
                        last_bet = (self.current_player + self.num_players - 1) % self.num_players;
                    }
                    _ => {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Error: not valid action",
                        ));
                    }
                }
            }

            let sum_folded: usize = self.folded.iter().map(|&b| b as usize).sum();
            if sum_folded == self.folded.len() - 1 {
                break;
            }

            if last_bet == self.current_player {
                break;
            }

            self.current_player = (self.current_player + 1) % self.num_players;
        }

        Ok(())
    }

    /// Advance to the next phase of the game
    pub fn advance_phase(&mut self, verbose: bool) -> PyResult<()> {
        if verbose {
            println!("End of {:?}", self.current_phase);
        }

        match self.current_phase {
            Phase::Preflop => {
                self.current_player = (self.dealer_pos + 1) % self.num_players;
                self.community_cards = (0..3)
                    .map(|_| self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty")))
                    .collect::<PyResult<Vec<_>>>()?;
                self.current_phase = Phase::Flop;
            }
            Phase::Flop => {
                self.current_player = (self.dealer_pos + 1) % self.num_players;
                let card = self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty"))?;
                self.community_cards.push(card);
                self.current_phase = Phase::Turn;
            }
            Phase::Turn => {
                self.current_player = (self.dealer_pos + 1) % self.num_players;
                let card = self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty"))?;
                self.community_cards.push(card);
                self.current_phase = Phase::River;
            }
            Phase::River => {
                self.current_phase = Phase::Showdown;
            }
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Error of phase"));
            }
        }

        Ok(())
    }

    /// Kill a player (when he has no stack left)
    pub fn kill(&mut self, player: usize) -> PyResult<()> { 
        self.stacks.remove(player);
        self.bets.remove(player);
        self.dead_agents.push(self.agents.remove(player));
        self.dead_names.push(self.names.remove(player));
        self.folded.remove(player);
        self.all_in.remove(player);
        self.rewards.remove(player);
        self.player_cards.remove(player);
        self.num_players -= 1;
        Ok(())
    }

    /// Determine winner(s) and conclude a game
    pub fn resolution(&mut self, verbose: bool) -> PyResult<()> {
        let mut scores: Vec<(String, Rank)> = Vec::new();
        let stacks_before_resolution = self.stacks.iter().sum::<i32>();

        let board = self.community_cards.join("");

        for i in 0..self.num_players {
            if !self.folded[i] {
                let player_cards = self.player_cards[i].clone().join("");
                let hand = Hand::new_from_str(&format!("{}{}", board, player_cards)).unwrap();
                let rank = hand.rank();
                scores.push((self.names[i].clone(), rank));
            }
        }

        scores.sort_by_key(|x| Reverse(x.1));

        let mut pots = vec![0];
        let mut pots_names: Vec<Vec<String>> = vec![vec![]];

        let sum_all_in: usize = self.all_in.iter().map(|&b| b as usize).sum();
        if sum_all_in == 0 {
            for i in 0..self.num_players {
                pots[0] += self.bets[i];

                if !self.folded[i] {
                    pots_names[0].push(self.names[i].clone())
                }
            }
        } else {
            let mut pot_index = 0;
            let mut bets = self.bets.clone();

            loop {
                let min = bets.iter()
                    .zip(self.folded.iter())
                    .enumerate()
                    .filter_map(|(_i, (&num, &flag))| {
                        if num != 0 && !flag {
                            Some(num)
                        } else {
                            None
                        }
                    })
                    .min();

                if let Some(val) = min {
                    for i in 0..self.num_players {
                        let n = std::cmp::min(val, bets[i]);
                        if n != 0 {
                            bets[i] -= n;
                            pots[pot_index] += n;

                            if !self.folded[i] {
                                pots_names[pot_index].push(self.names[i].clone());
                            }
                        }
                    }
                    pots.push(0);
                    pots_names.push(Vec::new());
                    pot_index += 1;
                } else {
                    break;
                }
            }
        }

        if verbose {
            println!("pots: {:?}\npots_player: {:?}", pots, pots_names);
        }

        // Distribute the pots
        let mut rest = 0;
        let mut i = 0;
        for p in pots {

            if p == 0 {
                continue;
            }

            // Determine pot winner(s)
            let mut winners = Vec::new();
            let mut rank: Option<Rank> = None;
            for (name, r) in scores.clone() {
                if pots_names[i].contains(&name) {
                    if winners.len() == 0 {
                        winners.push(name);
                        rank = Some(r);
                    } else {
                        if Some(r) == rank {
                            winners.push(name);
                        } else {
                            break;
                        }
                    }
                }
            }

            // Distribute gains
            rest += p % (winners.len() as i32);
            let takes = p / (winners.len() as i32);

            for j in 0..self.num_players {
                let agent_name = self.names[j as usize].clone();
                if winners.contains(&agent_name) {
                    self.stacks[j as usize] += takes;
                    if verbose {
                        println!("Winner pot {}: {}", i, agent_name);
                    }
                }
            }

            i += 1;
        }

        let mut j: i32 = 0;
        while (j as usize) < self.num_players {
            let agent_name = self.names[j as usize].clone();
            self.stacks[j as usize] -= self.bets[j as usize];
            if self.stacks[j as usize] == 0 {
                if verbose {
                    println!("{} lost", agent_name);
                }
                self.kill(j as usize)?;
                j -= 1;
            }
            j += 1;
        }

        if verbose {
            println!("State of stacks: {:?}", self.stacks);
            println!("{} player remaining", self.num_players);
        }

        if self.stacks.iter().sum::<i32>() + rest != stacks_before_resolution {
            panic!("Number of stack is not correct anymore!");
        }

        Ok(())
    }

    /// Revive all player to play another game
    pub fn revive(&mut self) -> PyResult<()> {
        for a in self.dead_agents.clone() {
            self.agents.push(a);
        };
        self.dead_agents = Vec::new();
        for n in self.dead_names.clone() {
            self.names.push(n)
        };
        self.dead_names = Vec::new();
        self.num_players = self.agents.len();

        self.stacks = vec![self.initial_stack; self.num_players];
        self.dealer_pos = 0;

        self.reset()?;

        Ok(())
    }

    /// play episode game(s) of poker
    pub fn play_game(&mut self, episode: i32, verbose: bool) -> PyResult<()> {
        let mut i = 1;

        while i <= episode {
            while self.num_players > 1 {
                self.reset()?;

                loop {
                    if i % 1000 == 0 {
                        println!("episode {} on {}", i, episode);
                    }

                    if verbose {
                        println!();
                        self.overall_state()?;
                    }
                    i += 1;

                    if self.folded.iter().filter(|&&b| b).count() != self.num_players - 1 {
                        self.step_bid(verbose)?;
                    }
                    self.advance_phase(verbose)?;

                    if self.current_phase == Phase::Showdown {
                        if verbose {
                            println!();
                            self.overall_state()?;
                        }

                        self.resolution(verbose)?;
                        break;
                    }
                }
            }
            self.revive()?;
        }

        Ok(())
    }
}

#[pymodule]
fn rust_poker_env(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Action>()?;
    m.add_class::<Phase>()?;
    m.add_class::<PokerEnv>()?;
    Ok(())
}