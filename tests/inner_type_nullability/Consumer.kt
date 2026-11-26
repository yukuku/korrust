fun consume(): Boolean = nullable(true) == null && nested(true) == null && value(false) == 4 && acceptNull() == -1
