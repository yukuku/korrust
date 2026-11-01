import unsignedreview.Token
fun probe(value: Token, step: suspend () -> Int): Int = value.apply(step)