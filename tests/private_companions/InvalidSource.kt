package companionhidden
class LocalVault {
    private companion object { const val SECRET: Int = 7 }
}
fun leaked(): Int = LocalVault.SECRET
