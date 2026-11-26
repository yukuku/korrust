class Owner { companion object { private fun secret(): Int = 2 } }
fun invalid() = Owner.secret()
