// hide generated values in private module
mod generator {
    include!(concat!(env!("OUT_DIR"), "/proto_gen.rs"));
}

use crate::net::outbe::fingerprint::v1::{
    compute_batch_fingerprint_request::Item, ComputeBatchFingerprintRequest, ComputeBatchFingerprintResponse,
    ComputeSingleFingerprintRequest, ComputeSingleFingerprintResponse,
};
use fingerprinting_core::{Fingerprint, FingerprintProtocol, TransactionFingerprintData};
use fingerprinting_types::RawTransaction;
use futures::stream::StreamExt;
use halo2_axiom::halo2curves::bn256::Fr;
use std::sync::Arc;
use tokio::sync::mpsc;
use volo_grpc::codegen::ReceiverStream;
use volo_grpc::{BoxStream, Code, Request, Response, Status};

pub use generator::proto_gen::*; // Reexport only subpackage from `proto_gen`

pub struct FingerprintService<P: FingerprintProtocol<Fr>> {
    protocol: Arc<P>,
}

impl<P: FingerprintProtocol<Fr> + Sync> FingerprintService<P> {
    pub fn new(protocol: P) -> FingerprintService<P> {
        FingerprintService {
            protocol: Arc::new(protocol),
        }
    }
}

impl<P: FingerprintProtocol<Fr> + Send + Sync + 'static>
    net::outbe::fingerprint::v1::FingerprintService for FingerprintService<P>
{
    async fn compute_single_fingerprint(
        &self,
        req: Request<ComputeSingleFingerprintRequest>,
    ) -> Result<Response<ComputeSingleFingerprintResponse>, Status> {
        let request = req.into_inner();
        let tx_data = request.transaction_data.ok_or(Status::new(
            Code::InvalidArgument,
            "Transaction data missing",
        ))?;
        let raw_tx: RawTransaction = tx_data.try_into()?;

        // preparing TransactionFingerprintData
        let raw_tx: TransactionFingerprintData<Fr> = raw_tx.try_into()?;

        // using the provided protocol built the fingerprint
        let fingerprint = raw_tx
            .complete_fingerprint(self.protocol.as_ref())
            .await
            .map_err(|e| {
                Status::new(
                    Code::Aborted,
                    format!("Failed to complete fingerprint computation: {}", e),
                )
            })?
            .into();

        let response = ComputeSingleFingerprintResponse {
            fingerprint: Some(fingerprint),
            _unknown_fields: Default::default(),
        };

        Ok(Response::new(response))
    }

    async fn compute_batch_fingerprint(
        &self,
        req: Request<ComputeBatchFingerprintRequest>,
    ) -> Result<Response<BoxStream<'static, Result<ComputeBatchFingerprintResponse, Status>>>, Status>
    {
        let request = req.into_inner();
        let tx_data = request.transaction_batch;
        let protocol = self.protocol.clone();

        let mut stream = futures::stream::iter(tx_data)
            .map(move |item: Item| {
                let protocol = protocol.clone();
                async move {
                    let item_id = item.item_id;
                    let raw_tx = item.transaction_data.ok_or(Status::new(
                        Code::InvalidArgument,
                        "Transaction data missing",
                    ))?;

                    let raw_tx: RawTransaction = raw_tx.try_into()?;

                    // preparing TransactionFingerprintData
                    let raw_tx: TransactionFingerprintData<Fr> = raw_tx.try_into()?;

                    // using the provided protocol built the fingerprint
                    let fingerprint = raw_tx
                        .complete_fingerprint(protocol.as_ref())
                        .await
                        .map_err(|e| {
                            Status::new(
                                Code::Aborted,
                                format!("Failed to complete fingerprint computation: {}", e),
                            )
                        })?
                        .into();

                    Ok(ComputeBatchFingerprintResponse {
                        item_id,
                        fingerprint: Some(fingerprint),
                        _unknown_fields: Default::default(),
                    })
                }
            })
            .buffer_unordered(16);

        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(resp) = stream.next().await {
                match tx.send(resp).await {
                    Ok(_) => {}
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

mod dto_convert {
    use crate::net;
    use anyhow::anyhow;
    use chrono::{DateTime, NaiveDate, Utc};
    use fingerprinting_core::Compact;
    use fingerprinting_types::{Money, RawTransaction, RawTransactionBuilder};
    use halo2_axiom::halo2curves::bn256::Fr;
    use pilota::FastStr;
    use volo_grpc::{Code, Status};

    impl TryInto<DateTime<Utc>> for net::outbe::common::v1::Timestamp {
        type Error = anyhow::Error;

        fn try_into(self) -> Result<DateTime<Utc>, Self::Error> {
            DateTime::from_timestamp(self.seconds as i64, self.nanos)
                .ok_or(anyhow!("Timestamp is not valid"))
        }
    }

    impl TryInto<NaiveDate> for net::outbe::common::v1::Date {
        type Error = anyhow::Error;

        fn try_into(self) -> Result<NaiveDate, Self::Error> {
            NaiveDate::from_ymd_opt(self.year as i32, self.month, self.day)
                .ok_or(anyhow!("Invalid or out-of-range date"))
        }
    }

    impl TryInto<Money> for net::outbe::common::v1::Money {
        type Error = anyhow::Error;

        fn try_into(self) -> Result<Money, Self::Error> {
            let currency = self.currency.to_string();
            let currency = currency
                .strip_prefix("CURRENCY_")
                .ok_or(anyhow!("Provided invalid currency {}", currency))?
                .to_string();

            Ok(Money {
                amount_base: self.units,
                amount_atto: self.atto,
                currency
            })
        }
    }

    impl TryInto<RawTransaction> for net::outbe::fingerprint::v1::TransactionFingerprintData {
        type Error = Status;

        fn try_into(self) -> Result<RawTransaction, Self::Error> {
            let tx_date_time = self.date_time.ok_or(Status::new(
                Code::InvalidArgument,
                "Transaction date time information is missing",
            ))?;
            let tx_wwd = self.wwd.ok_or(Status::new(
                Code::InvalidArgument,
                "Transaction WWD is missing",
            ))?;
            let tx_amount = self.amount.ok_or(Status::new(
                Code::InvalidArgument,
                "Transaction amount is missing",
            ))?;

            let date_time: DateTime<Utc> = tx_date_time.try_into()?;
            let wwd: NaiveDate = tx_wwd.try_into()?;
            let amount: Money = tx_amount.try_into()?;

            let raw_tx = RawTransactionBuilder::default()
                .bic(self.bic)
                .date_time(date_time)
                .wwd(wwd)
                .amount(amount)
                .build()
                .map_err(|e| {
                    Status::new(
                        Code::InvalidArgument,
                        format!("Failed to build transaction: {}", e),
                    )
                })?;

            Ok(raw_tx)
        }
    }

    impl From<Fr> for net::outbe::fingerprint::v1::Fingerprint {
        fn from(value: Fr) -> Self {
            net::outbe::fingerprint::v1::Fingerprint {
                fingerprint: pilota::Bytes::copy_from_slice(value.to_bytes().as_slice()),
                compact_fingerprint: FastStr::new(value.compact()),
                _unknown_fields: Default::default(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Utc};
    use lazy_static::lazy_static;
    use std::net::SocketAddr;
    use volo::FastStr;
    use fingerprinting_core::Compact;
    use super::*;

    lazy_static! {
        static ref CLIENT: net::outbe::fingerprint::v1::FingerprintServiceClient = {
            let addr: SocketAddr = "[::1]:9000".parse().unwrap();

            net::outbe::fingerprint::v1::FingerprintServiceClientBuilder::new(
                "fingerprinting-grpc-agent-client",
            )
            .address(addr)
            .build()
        };
    }
    #[tokio::test]
    pub async fn test_fingerprint_computation() -> Result<(), anyhow::Error> {
        let tx_date = Utc::now();

        let transaction_data = net::outbe::fingerprint::v1::TransactionFingerprintData {
            bic: FastStr::new("BCEELU21"),
            amount: Some(net::outbe::common::v1::Money {
                currency: net::outbe::common::v1::Currency::CURRENCY_EUR,
                units: 1000,
                atto: 0,
                _unknown_fields: Default::default(),
            }),
            date_time: Some(net::outbe::common::v1::Timestamp {
                seconds: tx_date.timestamp() as u64,
                nanos: tx_date.timestamp_subsec_nanos(),
                _unknown_fields: Default::default(),
            }),
            wwd: Some(net::outbe::common::v1::Date {
                year: tx_date.year() as u32,
                month: tx_date.month(),
                day: tx_date.day(),
                _unknown_fields: Default::default(),
            }),
            _unknown_fields: Default::default(),
        };

        println!("Transaction data: {:?}", transaction_data);
        println!("Requesting the fingerprint computation... from cooperative agents");

        let response = CLIENT
            .compute_single_fingerprint(ComputeSingleFingerprintRequest {
                transaction_data: Some(transaction_data),
                _unknown_fields: Default::default(),
            })
            .await?;

        let fingerprint = response.into_inner();
        let fingerprint = fingerprint.fingerprint.unwrap();
        let fixed_bytes = fingerprint.fingerprint.first_chunk::<32>().unwrap();

        let fr_fingerprint = Fr::from_bytes(fixed_bytes).unwrap();
        let compact_fingerprint = fingerprint.compact_fingerprint.to_string();

        println!(
            "Fingerprint: {} fr {}",
            compact_fingerprint,
            fr_fingerprint.compact()
        );

        Ok(())
    }
}
