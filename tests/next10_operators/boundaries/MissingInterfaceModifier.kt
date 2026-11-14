interface Bad { fun not(): Int }
fun invalid(value: Bad): Int = !value
