import referencevarargs.expose
fun bad() { val values=expose("x"); values[0]="wrong" }
