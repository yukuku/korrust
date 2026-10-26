package compatibility.annotations

@SourceMark("class")
class SourceRetention(
    @param:SourceMark("parameter")
    @field:SourceMark("field")
    @get:SourceMark("getter")
    @set:SourceMark("setter")
    @setparam:SourceMark("setter-parameter")
    var value: Int
) {
    @SourceMark("method")
    fun plus(delta: Int): Int = value + delta
}
