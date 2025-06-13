import time
start = time.time()

import random
from typing import List, Dict, Any
from enum import Enum
from treys import Evaluator, Card

# Énumérations pour clarifier les actions et les phases
class Action(Enum):
    FOLD = "fold"
    CHECK = "check"
    CALL = "call"
    RAISE = "raise"

class Phase(Enum):
    PREFLOP = "preflop"
    FLOP = "flop"
    TURN = "turn"
    RIVER = "river"

class PokerEnv:
    def __init__(self, agents: List[Any], small_blind: int = 10, big_blind: int = 20, initial_stack: int = 1000):
        """
        Initialise l'environnement de poker.
        
        Args:
            agents: Liste d'agents (objets avec une méthode choose_action)
            small_blind: Valeur de la petite blinde
            big_blind: Valeur de la grosse blinde
            initial_stack: Montant initial des jetons pour chaque joueur
        """
        self.agents = agents
        self.dead_agents = []
        self.num_players = len(agents)
        names = [f"player_{chr(65 + i)}" for i in range(self.num_players)]
        for i in range(len(agents)):
            agents[i].design_name(names[i])
        self.small_blind = small_blind
        self.big_blind = big_blind
        self.initial_stack = initial_stack
        
        # État du jeu
        self.stacks = [initial_stack for _ in range(self.num_players)]  # Jetons de chaque joueur
        self.dealer_pos = 0  # Position du donneur
        
        self.reset()

    def reset(self):
        """
        Réinitialise l'environnement pour une nouvelle main.
        """
        self.current_pot = 0 # Pot total
        self.bets = [0] * self.num_players # Mises actuelles par joueur dans le tour
        self.folded = [False] * self.num_players # Indique si un joueur s'est couché
        self.all_in = [False] * self.num_players # Indique si un joueur est all-in
        self.rewards = [0] * self.num_players # Store la reward de chaque joueur
        self.current_phase = Phase.PREFLOP # Phase actuelle
        self.dealer_pos = (self.dealer_pos + 1) % self.num_players
        self.current_player = (self.dealer_pos + 3) % self.num_players  # Après SB et BB
        
        # Créer et mélanger le paquet
        self.deck = [(rank, suit) for rank in ['2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', 'A']
                     for suit in ['h', 'd', 'c', 's']]  # H=Heart, D=Diamond, C=Club, S=Spade
        random.shuffle(self.deck)
        
        # Distribuer les cartes privatives
        self.player_cards = [[] for _ in range(self.num_players)]
        for i in range(self.num_players):
            self.player_cards[i] = [self.deck.pop(), self.deck.pop()]
        
        # Cartes communes
        self.community_cards = []
        
        # Forcer les blindes
        sb_pos = (self.dealer_pos + 1) % self.num_players
        bb_pos = (self.dealer_pos + 2) % self.num_players
        self.apply_bet(sb_pos, min(self.small_blind, self.stacks[sb_pos]))
        self.apply_bet(bb_pos, min(self.big_blind, self.stacks[bb_pos]))
        
    def apply_bet(self, player: int, amount: int):
        """
        Applique une mise pour un joueur.
        """
        self.bets[player] = amount
        if self.stacks[player]-self.bets[player] == 0:
            self.all_in[player] = True

    def kill(self, p):
        self.stacks.pop(p)
        self.bets.pop(p)
        self.dead_agents.append(self.agents.pop(p))
        self.folded.pop(p)
        self.all_in.pop(p)
        self.rewards.pop(p)
        self.num_players -= 1

    def revive(self):
        for a in self.dead_agents:
            self.agents.append(a)
        self.dead_agents = []
        self.num_players = len(self.agents)

        self.stacks = [self.initial_stack for _ in range(self.num_players)]  # Jetons de chaque joueur
        self.dealer_pos = 0

        self.reset()

    def get_available_actions(self) -> List[Action]:
        """
        Retourne les actions disponibles pour le joueur actuel.
        """
        actions = []
        current_bet = self.bets[self.current_player]
        max_bet = max(self.bets)  # Mise la plus haute à la table
        
        # Aucune action disponible si all in
        if self.all_in[self.current_player]:
            return actions
        
        if sum(self.all_in) + sum(self.folded) == len(self.folded)-1:
            if current_bet != max_bet:
                actions.append((Action.CALL, min(max_bet, self.stacks[self.current_player])))
            return actions

        # Toujours fold
        actions.append((Action.FOLD,))
        
        # Check si la mise actuelle du joueur est égale à la mise max
        if current_bet == max_bet:
            actions.append((Action.CHECK,))
        else:
            # Call si la mise max est supérieure à la mise actuelle
            actions.append((Action.CALL, min(max_bet, self.stacks[self.current_player])))
        
        # Raise si le joueur a assez de jetons (au moins le double de la dernière relance)
        if self.stacks[self.current_player] >= max_bet * 2:
            actions.append((Action.RAISE, (max_bet*2, self.stacks[self.current_player]))) # min,max
        
        return actions

    def get_state(self) -> Dict[str, Any]:
        """
        Retourne l'état observable du jeu pour le joueur actuel.
        """
        return {
            "player_cards": self.player_cards[self.current_player],
            "community_cards": self.community_cards,
            "stacks": self.stacks.copy(),
            "bets": self.bets.copy(),
            "pot": self.current_pot,
            "phase": self.current_phase,
            "current_player": self.current_player,
            "folded": self.folded.copy(),
            "all_in": self.all_in.copy()
        }
    
    def overall_state(self):
        """
        Print overall state for verbose
        """
        print(f"phase: {self.current_phase}")
        print(f"players_cards: {self.player_cards}")
        print(f"community_cards: {self.community_cards}")
        print(f"folded: {self.folded}")
        print(f"all_in: {self.all_in}")
        print(f"stacks: {self.stacks}")
        print(f"bets: {self.bets}")
        print(f"pot: {self.current_pot}")
    
    def step_bid(self, verbose=False):
        """
        Proceed 1 turn of bet
        """

        last_bet = (self.current_player + self.num_players - 1) % self.num_players

        while True:
            if self.folded[self.current_player]:
                if last_bet == self.current_player:
                    break

                self.current_player = (self.current_player + 1) % self.num_players
                continue

            agent = self.agents[self.current_player]

            state = self.get_state()
            available_actions = self.get_available_actions()

            if available_actions:
                action = agent.choose_action(state, available_actions)

                if verbose:
                    print(f"{self.agents[self.current_player].name} has {action}")

                match action[0]:
                    case Action.FOLD:
                        self.folded[self.current_player] = True
                    case Action.CHECK:
                        pass
                    case Action.CALL:
                        self.apply_bet(self.current_player, action[1])
                    case Action.RAISE:
                        self.apply_bet(self.current_player, action[1])
                        last_bet = (self.current_player + self.num_players - 1) % self.num_players
                    case _:
                        print("Error: not valid action")
                        raise "Error: not valid action"

            if sum(self.folded) == len(self.folded)-1:
                break

            if last_bet == self.current_player:
                break

            self.current_player = (self.current_player + 1) % self.num_players

    def advance_phase(self, verbose):
        """
        Avance à la phase suivante du jeu.
        """
        if verbose:
            print(f"End of {self.current_phase}")

        match self.current_phase:
            case Phase.PREFLOP:
                self.community_cards = [self.deck.pop() for _ in range(3)]  # Flop : 3 cards
                self.current_phase = Phase.FLOP

            case Phase.FLOP:
                self.community_cards.append(self.deck.pop())  # Turn : 1 card
                self.current_phase = Phase.TURN

            case Phase.TURN:
                self.community_cards.append(self.deck.pop())  # River : 1 card
                self.current_phase = Phase.RIVER

            case _:
                print("Error of phase")
                raise("Error of phase")
                        
    def to_treys(self, cards):
        """
        Return un array avec les cards en objet treys
        """
        return [Card.new(c) for c in cards]

    def resolution(self, verbose=False):
        """
        Détermine le gagnant et conclue la partie
        """

        winners = []
        scores = []

        if sum(self.folded) == len(self.folded) - 1:
            winners.append(self.agents[self.folded.index(False)].name)
        else:
            # random
            # num = random.randint(0, self.num_players-1)
            # winners.append(self.agents[num].name)

            board = self.to_treys(self.community_cards)
            evaluator = Evaluator()

            for i in range(self.num_players):
                hand = self.to_treys(self.player_cards[i])
                scores.append((self.agents[i].name, evaluator.evaluate(board, hand)))

            scores_sorted = sorted(scores, key=lambda x: x[1])
            winners.append(scores_sorted[0][0])

            min_score = scores_sorted[0][1]
            for i in range(1, self.num_players):
                if scores_sorted[i][1] == min_score:
                    winners.append(scores_sorted[i][0])
                else:
                    break

        self.current_pot += sum(self.bets)
        takes = self.current_pot // len(winners)
        self.current_pot = self.current_pot % len(winners)

        i=0
        while i < self.num_players:
        # répartir les gains et en déduire les rewards
            if self.agents[i].name in winners:
                self.stacks[i] += takes
                self.stacks[i] -= self.bets[i]
                if verbose:
                    print(f"Winner: {self.agents[i].name}")
            else:
                self.stacks[i] -= self.bets[i]
                if self.stacks[i] == 0:
                    if verbose:
                        print(f"{self.agents[i].name} is dead")
                    self.kill(i)
                    i-=1
            i+=1

        if verbose:
            print(f"State of stacks: {self.stacks}")
            print(f"number of player: {self.num_players}")


    def play_game(self, episode=1, verbose=False):
        """
        Joue une partie
        """
        i = 1

        while i <= episode:
            while self.num_players > 1:
                self.reset()

                while True: # Joue un round
                    if i%1000 == 0:
                        print(f"episode {i} on {episode}")

                    if verbose:
                        print()
                        self.overall_state()
                    i+=1

                    if not sum(self.folded) == len(self.folded)-1:
                        self.step_bid(verbose)
                    self.advance_phase(verbose)

                    if self.current_phase == Phase.RIVER:
                        if verbose:
                            print()
                            self.overall_state()

                        self.resolution(verbose)
                        break
            self.revive()

# # Exemple d'utilisation
class DummyAgent:
    def choose_action(self, state, available_actions):
        if not available_actions:
            return
        
        pick = random.choice(available_actions)

        if pick[0] == Action.RAISE:
            n = random.randint(pick[1][0], pick[1][1])
            return (Action.RAISE, n)
        return pick
    
    def learn(self):
        pass

    def design_name(self, name):
        self.name=name

agents = [DummyAgent() for _ in range(8)]
env = PokerEnv(agents, small_blind=10, big_blind=20, initial_stack=100)
env.play_game(verbose=False, episode=100000)

end = time.time()
print(f"Durée d'exécution : {end - start} secondes")
