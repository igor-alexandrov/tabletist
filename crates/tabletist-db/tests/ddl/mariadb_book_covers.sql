CREATE TABLE `book_covers` (
  `id` int(11) NOT NULL,
  `book_id` int(11) NOT NULL,
  `kind` varchar(16) NOT NULL,
  `image` blob DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `book_covers_book_id_kind` (`book_id`,`kind`),
  KEY `book_covers_book_id` (`book_id`),
  CONSTRAINT `book_covers_ibfk_1` FOREIGN KEY (`book_id`) REFERENCES `books` (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

-- Triggers
CREATE DEFINER=? TRIGGER book_covers_keep_kind BEFORE UPDATE ON book_covers FOR EACH ROW SET NEW.kind = LOWER(NEW.kind);
CREATE DEFINER=? TRIGGER book_covers_lower_kind BEFORE INSERT ON book_covers FOR EACH ROW SET NEW.kind = LOWER(NEW.kind);
