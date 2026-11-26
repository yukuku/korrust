import row.Column
fun name(): String = Column.name.name
fun title(): String = Column.title.name
fun index(value: Column): Int = value.ordinal
