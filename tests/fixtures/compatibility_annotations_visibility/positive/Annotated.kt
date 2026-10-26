package compatibility.annotations

@RuntimeMark(text = "type", count = 1, enabled = true)
@ClassMark("binary-type")
@SourceMark("source-type")
class Annotated(
    @param:RuntimeMark(text = "parameter", count = 2, enabled = false)
    @field:RuntimeMark(text = "field", count = 3, enabled = true)
    @get:RuntimeMark(text = "getter", count = 4, enabled = false)
    @set:RuntimeMark(text = "setter", count = 5, enabled = true)
    @setparam:RuntimeMark(text = "setter-parameter", count = 7, enabled = false)
    @field:ClassMark("binary-field")
    @get:ClassMark("binary-getter")
    @field:SourceMark("source-field")
    var value: Int
) {
    @RuntimeMark(text = "method", count = 6, enabled = false)
    fun plus(delta: Int): Int = value + delta
}
