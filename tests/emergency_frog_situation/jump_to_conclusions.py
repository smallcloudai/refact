# Picking up context, goal in this file:
# - pick up type of p
# - prioritize type definition over all the noise

import pygame
import numpy as np
import frog
from typing import Tuple


W = 640
H = 480


def draw_hello_frog(
    screen: pygame.Surface,
    message: str,
    color: Tuple[int, int, int] = (0, 255, 255),
    font_name: str = "Arial",
) -> None:
    font = pygame.font.SysFont(font_name, 32)
    text = font.render(message, True, color)
    text_rect = text.get_rect()
    text_rect.center = (W / 2, H / 2)
    screen.blit(text, text_rect)


creatures = [
    frog.Frog(
        np.random.uniform(0, W),
        np.random.uniform(0, H),
        np.random.uniform(-W/10, H/10),
        np.random.uniform(-W/10, H/10),
    ) for i in range(10)]


def main_loop():
    screen = pygame.display.set_mode((W,H))   # without space because it's a test it needs to pick up right line below
    quit_flag = False
    while not quit_flag:
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                quit_flag = True
        screen.fill((0, 0, 0))
        for p in creatures:
            pygame.draw.circle(screen, (0, 255, 0), (p.x, p.y), 10)
        draw_hello_frog(screen, "Jump To Conclusions!", (0, 200, 0))
        pygame.display.flip()
        pygame.time.Clock().tick(60)
        p: frog.Frog
        for p in creatures:
            p.jump(W, H)


if __name__ == '__main__':
    pygame.init()
    pygame.display.set_caption("Pond")
    main_loop()
    pygame.quit()
