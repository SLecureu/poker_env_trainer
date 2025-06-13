import time
start = time.time()

import random
from rust_poker_env import PokerEnv, Phase, Action

class DummyAgent:
    def choose_action(self, state, available_actions):
        if not available_actions:
            return
        
        pick = random.choice(available_actions)
        
        if pick[0] == "raise":
            n = random.randint(pick[1][0], pick[1][1])
            return ('raise', n)
        return pick
    
    def learn(self):
        pass

agents = [DummyAgent() for _ in range(8)]
env = PokerEnv(agents, small_blind=10, big_blind=20, initial_stack=100)
env.play_game(verbose=False, episode=100000)

end = time.time()
print(f"Durée d'exécution : {end - start} secondes")
