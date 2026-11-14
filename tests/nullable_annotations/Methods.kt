package nullableannotations

@RuntimeMark("top")
@BinaryMark("top")
@JvmOverloads
fun top(text: String? = null, force: Boolean = false): String? = if (force) "forced" else text

class Member {
    @RuntimeMark("member")
    @BinaryMark("member")
    @JvmOverloads
    fun member(text: String? = null, force: Boolean = false): String? = if (force) "forced" else text

    companion object {
        @RuntimeMark("companion")
        @BinaryMark("companion")
        @JvmStatic
        @JvmOverloads
        fun companion(text: String? = null, force: Boolean = false): String? = if (force) "forced" else text

        @RuntimeMark("plain")
        @BinaryMark("plain")
        @JvmOverloads
        fun plain(text: String? = null, force: Boolean = false): String? = if (force) "forced" else text
    }
}

object Holder {
    @RuntimeMark("static")
    @BinaryMark("static")
    @JvmStatic
    @JvmOverloads
    fun static(text: String? = null, force: Boolean = false): String? = if (force) "forced" else text

    @RuntimeMark("instance")
    @BinaryMark("instance")
    @JvmOverloads
    fun instance(text: String? = null, force: Boolean = false): String? = if (force) "forced" else text
}
