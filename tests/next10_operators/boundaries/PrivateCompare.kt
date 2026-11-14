class Bad { private operator fun compareTo(other: Bad): Int = 0 }
fun invalid(): Boolean = Bad() < Bad()
