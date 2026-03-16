	.text
	.globl main
main:
	li	t5, 2
	li	t6, 3
	mul	a7, t5, t6
	li	t5, 1
	add	a6, t5, a7
	mv a0, a6
	ret
