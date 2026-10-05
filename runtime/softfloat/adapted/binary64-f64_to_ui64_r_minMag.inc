
/*============================================================================

This C source file is part of the SoftFloat IEEE Floating-Point Arithmetic
Package, Release 3e, by John R. Hauser.

Copyright 2011, 2012, 2013, 2014, 2015, 2016 The Regents of the University of
California.  All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

 1. Redistributions of source code must retain the above copyright notice,
    this list of conditions, and the following disclaimer.

 2. Redistributions in binary form must reproduce the above copyright notice,
    this list of conditions, and the following disclaimer in the documentation
    and/or other materials provided with the distribution.

 3. Neither the name of the University nor the names of its contributors may
    be used to endorse or promote products derived from this software without
    specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS "AS IS", AND ANY
EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE, ARE
DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

=============================================================================*/


uint_fast64_t cosmic_sf_f64_to_u64( cosmic_sf_context *ctx, unsigned long long a )
{
    uint_fast64_t uiA;
    int_fast16_t exp;
    uint_fast64_t sig;
    int_fast16_t shiftDist;
    bool sign;
    uint_fast64_t z;

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    uiA = a;
    exp = expF64UI( uiA );
    sig = fracF64UI( uiA );
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    shiftDist = 0x433 - exp;
    if ( 53 <= shiftDist ) {
        if ( true && (exp | sig) ) {
            ctx->flags |= cosmic_sf_private_flag_inexact;
        }
        return 0;
    }
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    sign = signF64UI( uiA );
    if ( sign ) goto invalid;
    if ( shiftDist <= 0 ) {
        if ( shiftDist < -11 ) goto invalid;
        z = (sig | UINT64_C( 0x0010000000000000 ))<<-shiftDist;
    } else {
        sig |= UINT64_C( 0x0010000000000000 );
        z = sig>>shiftDist;
        if ( true && (uint64_t) (sig<<(-shiftDist & 63)) ) {
            ctx->flags |= cosmic_sf_private_flag_inexact;
        }
    }
    return z;
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
 invalid:
    cosmic_sf_raise( ctx, cosmic_sf_private_flag_invalid );
    return
        (exp == 0x7FF) && sig ? ui64_fromNaN
            : sign ? ui64_fromNegOverflow : ui64_fromPosOverflow;

}

