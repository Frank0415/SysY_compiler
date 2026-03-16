	.text
	.globl main
main:
	li	t5, 1
	li	t6, 2
	sgt	a7, t5, t6
	seqz	a7, a7
	mv a0, a7
	ret
