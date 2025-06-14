use pyo3::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use pyo3::types::{PyDict, PyTuple};
use pyo3::ToPyObject;
use rs_poker::core::{Hand, Rankable, Rank};

// Assuming Action and Phase enums are already defined as in the previous response
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
}

impl ToPyObject for Phase {
    fn to_object(&self, py: Python) -> PyObject {
        match self {
            Phase::Preflop => "preflop".to_object(py),
            Phase::Flop => "flop".to_object(py),
            Phase::Turn => "turn".to_object(py),
            Phase::River => "river".to_object(py),
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
    initial_stack: i32,
    #[pyo3(get, set)]
    stacks: Vec<i32>,
    #[pyo3(get, set)]
    dealer_pos: usize,
    #[pyo3(get, set)]
    current_pot: i32,
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
    /// init poker env
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
            initial_stack,
            stacks: vec![initial_stack; num_players],
            dealer_pos: 0,
            current_pot: 0,
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
        self.current_pot = 0;
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
        let max_bet = self.bets.iter().max().copied().unwrap_or(0);

        // No action if all in
        if self.all_in[self.current_player] {
            return Ok(actions);
        };

        let sum_all_in: usize = self.all_in.iter().map(|&b| b as usize).sum();
        let sum_folded: usize = self.folded.iter().map(|&b| b as usize).sum();

        if sum_all_in + sum_folded == self.folded.len() - 1 {
            if current_bet != max_bet {
                let call_amount = max_bet.min(self.stacks[self.current_player]);
                Python::with_gil(|py| {
                    actions.push(PyTuple::new_bound(py, [Action::Call.to_object(py), call_amount.to_object(py)]).into());
                });
            }
            return Ok(actions)
        };

        // Always fold
        Python::with_gil(|py| {
            actions.push(PyTuple::new_bound(py, [Action::Fold.to_object(py)]).into());
        });

        // "Check" is the bet of the player is equal to the max_bet, "Call" if not
        if current_bet == max_bet {
            Python::with_gil(|py| {
                actions.push(PyTuple::new_bound(py, [Action::Check.to_object(py)]).into());
            });
        } else {
            let call_amount = max_bet.min(self.stacks[self.current_player]);
            Python::with_gil(|py| {
                actions.push(PyTuple::new_bound(py, [Action::Call.to_object(py), call_amount.to_object(py)]).into());
            });
        };

        if self.stacks[self.current_player] >= max_bet*2 {
            let raise_range = (max_bet * 2, self.stacks[self.current_player]);
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
            dict.set_item("pot", self.current_pot)?;
            dict.set_item("phase", &self.current_phase)?;
            dict.set_item("current_player", self.current_player)?;
            dict.set_item("folded", self.folded.clone())?;
            dict.set_item("all_in", self.all_in.clone())?;
            Ok(dict.into())
        })
    }

    /// Print overall state
    pub fn overall_state(&mut self) -> PyResult<()> {
        println!("phase: {0:?}\nplayers_cards: {1:?}\ncommunity_cards: {2:?}\nfolded: {3:?}')\nall_in: {4:?}\nstacks: {5:?}\nbets: {6:?}\npot: {7}\n",
                    self.current_phase,
                    self.player_cards,
                    self.community_cards,
                    self.folded,
                    self.all_in,
                    self.stacks,
                    self.bets,
                    self.current_pot);
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

            if self.folded.iter().filter(|&&b| b).count() == self.num_players - 1 {
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
                self.community_cards = (0..3)
                    .map(|_| self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty")))
                    .collect::<PyResult<Vec<_>>>()?;
                self.current_phase = Phase::Flop;
            }
            Phase::Flop => {
                let card = self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty"))?;
                self.community_cards.push(card);
                self.current_phase = Phase::Turn;
            }
            Phase::Turn => {
                let card = self.deck.pop().ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Deck is empty"))?;
                self.community_cards.push(card);
                self.current_phase = Phase::River;
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
        self.num_players -= 1;
        Ok(())
    }

    /// Determine winner(s) and conclude a game
    pub fn resolution(&mut self, verbose: bool) -> PyResult<()> {
        let mut winners: Vec<String> = Vec::new();
        let mut scores: Vec<(String, Rank)> = Vec::new();

        // Check if only one player hasn't folded
        if self.folded.iter().filter(|&&b| b).count() == self.num_players - 1 {
            if let Some(index) = self.folded.iter().position(|&b| !b) {
                winners.push(self.names[index].clone());
            }
        } else {

            let board = self.community_cards.join("");

            for i in 0..self.num_players {
                let player_cards = self.player_cards[i].clone().join("");
                let hand = Hand::new_from_str(&format!("{}{}", board, player_cards)).unwrap();
                let rank = hand.rank();
                scores.push((self.names[i].clone(), rank));
            }

            scores.sort_by_key(|x| x.1);
            winners.push(scores[0].0.clone());

            let min_score = scores[0].1;
            for i in 1..self.num_players {
                if scores[i].1 == min_score {
                    winners.push(scores[i].0.clone());
                } else {
                    break;
                }
            }
        }

        // Distribute the pot
        self.current_pot += self.bets.iter().sum::<i32>();
        println!("{:?}", self.current_pot);
        let takes = self.current_pot / (winners.len() as i32);
        println!("{:?}", winners);
        println!("{:?}", winners.len());
        println!("{:?}", takes);
        self.current_pot = self.current_pot % (winners.len() as i32);
        println!("{:?}", self.current_pot);

        let mut i = 0;
        while i < self.num_players {
            let agent_name = self.names[i].clone();
            if winners.contains(&agent_name) {
                self.stacks[i] += takes;
                self.stacks[i] -= self.bets[i];
                if verbose {
                    println!("Winner: {}", agent_name);
                }
            } else {
                self.stacks[i] -= self.bets[i];
                if self.stacks[i] == 0 {
                    if verbose {
                        println!("{} lost", agent_name);
                    }
                    self.kill(i)?;
                    i = i.saturating_sub(1);
                }
            }
            i += 1;
        }

        if verbose {
            println!("State of stacks: {:?}", self.stacks);
            println!("{} player remaining", self.num_players);
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

                    if self.current_phase == Phase::River {
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