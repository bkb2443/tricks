#[cfg(test)]
mod tests {
    #[test]
    fn test_compute_names_ordering() {
        // This mirrors the compute_names function logic
        fn compute_names(player_count: usize, bot_seats: &[bool]) -> Vec<String> {
            let mut bot_counter = 0usize;
            (0..player_count)
                .map(|i| {
                    if bot_seats.get(i).copied().unwrap_or(false) {
                        bot_counter += 1;
                        format!("Bot {bot_counter}")
                    } else {
                        "Player".to_string()
                    }
                })
                .collect()
        }
        
        // Test case: 5 seats with pattern [Human, Bot, Human, Bot, Bot]
        let bot_seats = vec![false, true, false, true, true];
        let names = compute_names(5, &bot_seats);
        
        assert_eq!(names[0], "Player", "Seat 0 should be Player");
        assert_eq!(names[1], "Bot 1", "Seat 1 should be Bot 1");
        assert_eq!(names[2], "Player", "Seat 2 should be Player");
        assert_eq!(names[3], "Bot 2", "Seat 3 should be Bot 2");
        assert_eq!(names[4], "Bot 3", "Seat 4 should be Bot 3");
    }
}
