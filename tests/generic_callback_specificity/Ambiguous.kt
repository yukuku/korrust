package ranked
fun ambiguous(a: Child, b: Child): Int = crossed(a, b) { _, _ -> 1 }
