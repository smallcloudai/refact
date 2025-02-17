
# ERROR py_body syntax error: "comment" in # Picking up context, goal in this file:
# Picking up context, goal in this file:
# ERROR py_body syntax error: "comment" in # - pick up type of p
# - pick up type of p
# ERROR py_body syntax error: "comment" in # - prioritize type definition over all the noise
# - prioritize type definition over all the noise

import pygame
import numpy as np
import frog
from typing import Tuple


# v W int
# U{ go_up root::W }
W = 640
# v H int
# U{ go_up root::H }
H = 480


# f draw_hello_frog() void
def draw_hello_frog(
    # p screen ?::Surface
    # U{ alias ?::pygame } U{ attr guess ?::Surface }
    screen: pygame.Surface,
    # p message str
    message: str,
    # p color (int,int,int)
    color: Tuple[int, int, int] = (0, 255, 255),
    # p font_name str
    font_name: str = "Arial",
) -> None:
    # v font ERR/FUNC_NOT_FOUND/?::SysFont
    # U{ alias ?::pygame } U{ attr guess ?::font } U{ attr guess ?::SysFont } U{ go_up root::draw_hello_frog::font_name } U{ go_up root::draw_hello_frog::font }
    font = pygame.font.SysFont(font_name, 32)
    # v text ERR/FUNC_NOT_FOUND/?::render
    # U{ go_up root::draw_hello_frog::font } U{ attr guess ?::render } U{ go_up root::draw_hello_frog::message } U{ go_up root::draw_hello_frog::color } U{ go_up root::draw_hello_frog::text }
    text = font.render(message, True, color)
    # v text_rect ERR/FUNC_NOT_FOUND/?::get_rect
    # U{ go_up root::draw_hello_frog::text } U{ attr guess ?::get_rect } U{ go_up root::draw_hello_frog::text_rect }
    text_rect = text.get_rect()
    # ERROR py_var_add cannot create: "attribute" in text_rect.center
    # U{ go_up root::W } U{ go_up root::H } U{ go_up root::draw_hello_frog::text_rect } U{ attr guess ?::center }
    text_rect.center = (W / 2, H / 2)
    # U{ go_up root::draw_hello_frog::screen } U{ attr guess ?::blit } U{ go_up root::draw_hello_frog::text } U{ go_up root::draw_hello_frog::text_rect }
    screen.blit(text, text_rect)


# v creatures [ERR/FUNC_NOT_FOUND/?::Frog]
# U{ go_up root::creatures }
creatures = [
    # U{ alias ?::frog } U{ attr guess ?::Frog }
    frog.Frog(
        # U{ alias ?::numpy } U{ attr guess ?::random } U{ attr guess ?::uniform } U{ go_up root::W }
        np.random.uniform(0, W),
        # U{ alias ?::numpy } U{ attr guess ?::random } U{ attr guess ?::uniform } U{ go_up root::H }
        np.random.uniform(0, H),
        # U{ alias ?::numpy } U{ attr guess ?::random } U{ attr guess ?::uniform } U{ go_up root::W } U{ go_up root::H }
        np.random.uniform(-W/10, H/10),
        # U{ alias ?::numpy } U{ attr guess ?::random } U{ attr guess ?::uniform } U{ go_up root::W } U{ go_up root::H }
        np.random.uniform(-W/10, H/10),
    # v i int
    # U{ go_up root::<listcomp>::i }
    ) for i in range(10)]


# f main_loop() !void
def main_loop():
    # ERROR py_body syntax error: "comment" in # without space because it's a test it needs to pi...
    # v screen ERR/FUNC_NOT_FOUND/?::set_mode
    # U{ alias ?::pygame } U{ attr guess ?::display } U{ attr guess ?::set_mode } U{ go_up root::W } U{ go_up root::H } U{ go_up root::main_loop::screen }
    screen = pygame.display.set_mode((W,H))   # without space because it's a test it needs to pick up the correct line below
    # v quit_flag bool
    # U{ go_up root::main_loop::quit_flag }
    quit_flag = False
    # U{ go_up root::main_loop::quit_flag }
    while not quit_flag:
        # v event ERR/FUNC_NOT_FOUND/?::get
        # U{ alias ?::pygame } U{ attr guess ?::event } U{ attr guess ?::get } U{ go_up root::main_loop::event }
        for event in pygame.event.get():
            # U{ go_up root::main_loop::event } U{ attr guess ?::type } U{ alias ?::pygame } U{ attr guess ?::QUIT }
            if event.type == pygame.QUIT:
                # U{ go_up root::main_loop::quit_flag }
                quit_flag = True
        # U{ go_up root::main_loop::screen } U{ attr guess ?::fill }
        screen.fill((0, 0, 0))
        # U{ go_up root::creatures } U{ go_up root::main_loop::p }
        for p in creatures:
            # U{ alias ?::pygame } U{ attr guess ?::draw } U{ attr guess ?::circle } U{ go_up root::main_loop::screen } U{ go_up root::main_loop::p } U{ attr guess ?::x } U{ go_up root::main_loop::p } U{ attr guess ?::y }
            pygame.draw.circle(screen, (0, 255, 0), (p.x, p.y), 10)
        # U{ go_up root::draw_hello_frog } U{ go_up root::main_loop::screen }
        draw_hello_frog(screen, "Jump To Conclusions!", (0, 200, 0))
        # v p ?::Frog
        # U{ alias ?::frog } U{ attr guess ?::Frog } U{ go_up root::main_loop::p }
        p: frog.Frog
        # U{ go_up root::creatures } U{ go_up root::main_loop::p }
        for p in creatures:
            # U{ go_up root::main_loop::p } U{ attr guess ?::jump } U{ go_up root::W } U{ go_up root::H }
            p.jump(W, H)
        # U{ alias ?::pygame } U{ attr guess ?::display } U{ attr guess ?::flip }
        pygame.display.flip()
        # U{ alias ?::pygame } U{ attr guess ?::time } U{ attr guess ?::Clock } U{ attr guess ?::tick }
        pygame.time.Clock().tick(60)


if __name__ == '__main__':
    # U{ alias ?::pygame } U{ attr guess ?::init }
    pygame.init()
    # U{ alias ?::pygame } U{ attr guess ?::display } U{ attr guess ?::set_caption }
    pygame.display.set_caption("Pond")
    # U{ go_up root::main_loop }
    main_loop()
    # U{ alias ?::pygame } U{ attr guess ?::quit }
    pygame.quit()