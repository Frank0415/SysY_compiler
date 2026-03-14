	.text
	.globl main
main:
	li	t5, 6
	xor	a7, t5, x0
	seqz	a7, a7
	sub	a6, x0, a7
	sub	a7, x0, a6
	mv a0, a7
	ret
