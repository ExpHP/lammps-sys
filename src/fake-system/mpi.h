// Part of `not(feature = "system-mpi")`
//
// This file exists so that you don't *need* mpi.h on your system path
// if you don't plan to call the one function that needs it.
//
// On the rust side of things, this type is ignored and substituted
// with an empty enum.
struct MPI_Comm {};
